use crate::{
    config::YekConfig,
    models::{InputConfig, OutputConfig, ProcessingConfig, RepositoryInfo},
    pipeline::ProcessingPipeline,
    repository::{get_repository_factory, RealFileSystem},
};
use anyhow::Result;
use bytesize::ByteSize;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, Level};
use tracing_subscriber::fmt;

/// New main function using the improved architecture
pub fn main_new() -> Result<()> {
    // 1) Parse CLI + config files using existing YekConfig:
    let full_config = YekConfig::init_config();

    let env_filter = if full_config.debug {
        "yek=debug,ignore=off"
    } else {
        "yek=info,ignore=off"
    };

    // 2) Initialize tracing:
    fmt::Subscriber::builder()
        .with_max_level(if full_config.debug {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
        .with_env_filter(env_filter)
        .compact()
        .init();

    if full_config.debug {
        let config_str = serde_json::to_string_pretty(&full_config)?;
        debug!("Configuration:\n{}", config_str);
    }

    // 3) Convert old config to new config structures
    let (input_config, output_config, processing_config) = convert_config(&full_config)?;

    // 4) Create repository info
    let repository_factory = get_repository_factory();
    let repository_info = match repository_factory.create_repository_info(
        Path::new(&full_config.input_paths[0]), // Use first input path as base
        &input_config,
    ) {
        Ok(repo_info) => repo_info,
        Err(e) => {
            eprintln!("Warning: Failed to create repository info: {}", e);
            RepositoryInfo::new(Path::new(".").to_path_buf(), false)
        }
    };

    // 5) Create processing context
    let file_system = Arc::new(RealFileSystem);
    let git_operations = if repository_info.is_git_repo {
        // Try to create git operations
        match crate::repository::RealGitOperations::new(&repository_info.root_path) {
            Ok(git_ops) => Some(Arc::new(git_ops) as Arc<dyn crate::repository::GitOperations>),
            Err(e) => {
                debug!("Failed to create git operations: {}", e);
                None
            }
        }
    } else {
        None
    };

    let context = crate::pipeline::ProcessingContext::new(
        input_config,
        output_config,
        processing_config,
        repository_info,
        file_system,
    );

    // 6) Process using the new pipeline
    let pipeline = ProcessingPipeline::new(context.clone(), git_operations);

    match pipeline.process() {
        Ok(files) => {
            // 7) Generate output using the new architecture
            let output = generate_output(&files, &context)?;

            // 8) Handle output based on streaming mode
            if full_config.stream {
                handle_streaming_output(&output, &full_config)?;
            } else {
                handle_file_output(&output, &files, &full_config)?;
            }

            if full_config.debug {
                debug!("{} files processed", files.len());
                debug!("Output lines: {}", output.lines().count());
            }

            Ok(())
        }
        Err(e) => {
            eprintln!("Error during processing: {}", e);
            std::process::exit(1);
        }
    }
}

/// Convert old YekConfig to new config structures
fn convert_config(old_config: &YekConfig) -> Result<(InputConfig, OutputConfig, ProcessingConfig)> {
    // Convert input configuration
    let input_config = InputConfig {
        input_paths: old_config.input_paths.clone(),
        ignore_patterns: old_config
            .ignore_patterns
            .iter()
            .map(|p| glob::Pattern::new(p).unwrap_or_else(|_| glob::Pattern::new("*").unwrap()))
            .collect(),
        binary_extensions: old_config.binary_extensions.iter().cloned().collect(),
        max_git_depth: old_config.max_git_depth,
        git_boost_max: old_config.git_boost_max,
    };

    // Convert output configuration
    let output_config = OutputConfig {
        max_size: old_config.max_size.clone(),
        token_mode: old_config.token_mode,
        token_limit: if old_config.token_mode {
            Some(old_config.tokens.clone())
        } else {
            None
        },
        output_template: old_config.output_template.clone().unwrap_or_default(),
        line_numbers: old_config.line_numbers,
        json_output: old_config.json,
        tree_header: old_config.tree_header,
        tree_only: old_config.tree_only,
        output_dir: old_config.output_dir.clone(),
        output_name: old_config.output_name.clone(),
        stream: old_config.stream,
    };

    // Convert processing configuration
    let processing_config = ProcessingConfig {
        priority_rules: old_config.priority_rules.clone(),
        debug: old_config.debug,
        parallel: true, // Always use parallel processing in new architecture
        max_threads: None,
        memory_limit_mb: None,
        batch_size: 1000,
    };

    Ok((input_config, output_config, processing_config))
}

/// Generate output from processed files using the new architecture
fn generate_output(
    files: &[crate::models::ProcessedFile],
    context: &crate::pipeline::ProcessingContext,
) -> Result<String> {
    if context.output_config.tree_only {
        // Generate tree-only output
        let file_paths: Vec<std::path::PathBuf> = files
            .iter()
            .map(|f| std::path::PathBuf::from(&f.rel_path))
            .collect();
        return Ok(crate::tree::generate_tree(&file_paths));
    }

    // Generate tree header if requested
    let tree_header = if context.output_config.tree_header {
        let file_paths: Vec<std::path::PathBuf> = files
            .iter()
            .map(|f| std::path::PathBuf::from(&f.rel_path))
            .collect();
        crate::tree::generate_tree(&file_paths)
    } else {
        String::new()
    };

    // Filter files based on size limits
    let mut accumulated = 0usize;
    let cap = if context.output_config.token_mode {
        crate::parse_token_limit(
            &context
                .output_config
                .token_limit
                .clone()
                .unwrap_or_default(),
        )?
    } else {
        ByteSize::from_str(&context.output_config.max_size)
            .unwrap_or_else(|_| ByteSize::from_str("10MB").unwrap())
            .as_u64() as usize
    };

    // Account for tree header size
    let tree_header_size = if context.output_config.tree_header {
        if context.output_config.token_mode {
            crate::count_tokens(&tree_header)
        } else {
            tree_header.len()
        }
    } else {
        0
    };
    accumulated += tree_header_size;

    let mut files_to_include = Vec::new();
    for file in files {
        let file_copy = file.clone();
        let content_size = file_copy.get_size(
            context.output_config.token_mode,
            context.output_config.line_numbers,
        );

        if accumulated + content_size <= cap {
            accumulated += content_size;
            files_to_include.push(file);
        } else {
            break;
        }
    }

    // Generate main content
    let main_content = if context.output_config.json_output {
        generate_json_output(&files_to_include, context)?
    } else {
        generate_template_output(&files_to_include, context)?
    };

    // Combine tree header with main content
    if context.output_config.tree_header {
        Ok(format!("{}{}", tree_header, main_content))
    } else {
        Ok(main_content)
    }
}

/// Generate JSON output
fn generate_json_output(
    files: &[&crate::models::ProcessedFile],
    context: &crate::pipeline::ProcessingContext,
) -> Result<String> {
    let json_objects: Vec<_> = files
        .iter()
        .map(|f| {
            let content = f.get_formatted_content(context.output_config.line_numbers);
            serde_json::json!({
                "filename": f.rel_path,
                "content": content,
            })
        })
        .collect();

    serde_json::to_string_pretty(&json_objects)
        .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))
}

/// Generate template-based output
fn generate_template_output(
    files: &[&crate::models::ProcessedFile],
    context: &crate::pipeline::ProcessingContext,
) -> Result<String> {
    let mut output_parts = Vec::new();

    for file in files {
        let content = file.get_formatted_content(context.output_config.line_numbers);
        let formatted = context
            .output_config
            .output_template
            .replace("FILE_PATH", &file.rel_path)
            .replace("FILE_CONTENT", content)
            // Handle both literal "\n" and escaped "\\n"
            .replace("\\\\\n", "\n") // First handle escaped newline
            .replace("\\\\n", "\n"); // Then handle escaped \n sequence

        output_parts.push(formatted);
    }

    Ok(output_parts.join("\n"))
}

/// Handle streaming output
fn handle_streaming_output(output: &str, config: &YekConfig) -> Result<()> {
    if let Some(output_name) = &config.output_name {
        std::fs::write(output_name, output.as_bytes())?;
        println!("{}", output_name);
    } else {
        println!("{}", output);
    }
    Ok(())
}

/// Handle file output with checksum
fn handle_file_output(
    output: &str,
    _files: &[crate::models::ProcessedFile],
    config: &YekConfig,
) -> Result<()> {
    let checksum = YekConfig::get_checksum(&config.input_paths);

    let final_path = if let Some(output_name) = &config.output_name {
        output_name.clone()
    } else {
        let extension = if config.json { "json" } else { "txt" };
        let output_dir = config.output_dir.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Output directory is required when not in streaming mode.")
        })?;

        Path::new(output_dir)
            .join(format!("yek-output-{}.{}", checksum, extension))
            .to_string_lossy()
            .to_string()
    };

    std::fs::write(&final_path, output.as_bytes())?;
    println!("{}", final_path);

    Ok(())
}

/// Temporary bridge to maintain backward compatibility
/// This function delegates to the new architecture
pub fn serialize_repo_new(
    config: &YekConfig,
) -> Result<(String, Vec<crate::parallel::ProcessedFile>)> {
    // Convert old config to new config structures
    let (input_config, output_config, processing_config) = convert_config(config)?;

    // Create repository info
    let repository_factory = get_repository_factory();
    let repository_info = repository_factory
        .create_repository_info(Path::new(&config.input_paths[0]), &input_config)?;

    // Create processing context
    let file_system = Arc::new(RealFileSystem);
    let git_operations = if repository_info.is_git_repo {
        match crate::repository::RealGitOperations::new(&repository_info.root_path) {
            Ok(git_ops) => Some(Arc::new(git_ops) as Arc<dyn crate::repository::GitOperations>),
            Err(_) => None,
        }
    } else {
        None
    };

    let context = crate::pipeline::ProcessingContext::new(
        input_config,
        output_config,
        processing_config,
        repository_info,
        file_system,
    );

    // Process using the new pipeline
    let pipeline = ProcessingPipeline::new(context.clone(), git_operations);
    let files = pipeline.process()?;

    // Generate output
    let output = generate_output(&files, &context)?;

    // Convert back to old ProcessedFile format for compatibility
    let old_files = files
        .into_iter()
        .map(|f| crate::parallel::ProcessedFile {
            priority: f.priority,
            file_index: f.file_index,
            rel_path: f.rel_path,
            content: f.content,
        })
        .collect();

    Ok((output, old_files))
}
