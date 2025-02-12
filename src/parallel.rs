// Build the gitignore
    let mut gitignore_builder = GitignoreBuilder::new(base_dir);
    // Add our custom patterns first
    for pattern in &config.ignore_patterns {
        gitignore_builder.add_line(None, pattern)?;
    }

    // If there is a .gitignore in this folder, add it last so its "!" lines override prior patterns
    let gitignore_file = base_dir.join(".gitignore");
    if gitignore_file.exists() {
        gitignore_builder.add(&gitignore_file)?;
    }

    let gitignore = Arc::new(gitignore_builder.build()?);