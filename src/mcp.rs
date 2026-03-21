use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use yek::{config::YekConfig, serialize_repo};

/// MCP (Model Context Protocol) server for yek.
/// Exposes repository serialization as an MCP tool over stdio.

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

fn handle_request(request: &JsonRpcRequest) -> Value {
    match request.method.as_str() {
        "initialize" => handle_initialize(),
        "notifications/initialized" => return Value::Null, // No response for notifications
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(&request.params),
        _ => json!({
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", request.method)
            }
        }),
    }
}

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "yek",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "serialize_repo",
                "description": "Serialize a repository or directory into a single text output optimized for LLM consumption. Uses git history for smart file prioritization and respects .gitignore patterns.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the repository or directory to serialize. Defaults to current directory."
                        },
                        "max_size": {
                            "type": "string",
                            "description": "Maximum output size (e.g. '10MB', '500KB'). Defaults to '10MB'."
                        },
                        "ignore_patterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Additional glob patterns to ignore (e.g. ['tests/**', '*.log'])."
                        },
                        "tokens": {
                            "type": "string",
                            "description": "Maximum number of tokens (e.g. '128000'). When set, uses token-based sizing instead of byte-based."
                        }
                    }
                }
            }
        ]
    })
}

fn handle_tools_call(params: &Value) -> Value {
    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match tool_name {
        "serialize_repo" => handle_serialize_repo(params.get("arguments").unwrap_or(&json!({}))),
        _ => json!({
            "isError": true,
            "content": [{
                "type": "text",
                "text": format!("Unknown tool: {}", tool_name)
            }]
        }),
    }
}

fn handle_serialize_repo(args: &Value) -> Value {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".")
        .to_string();

    let max_size = args
        .get("max_size")
        .and_then(|v| v.as_str())
        .unwrap_or("10MB")
        .to_string();

    let tokens = args
        .get("tokens")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let ignore_patterns: Vec<String> = args
        .get("ignore_patterns")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut config = YekConfig {
        input_paths: vec![path],
        max_size,
        tokens,
        stream: true, // Always stream for MCP (no file output)
        ignore_patterns,
        ..Default::default()
    };

    // Merge default ignore patterns
    let mut merged_ignore = yek::defaults::DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    merged_ignore.extend(config.ignore_patterns.drain(..));
    config.ignore_patterns = merged_ignore;

    // Merge default binary extensions
    let mut merged_bins = yek::defaults::BINARY_FILE_EXTENSIONS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    merged_bins.extend(config.binary_extensions.drain(..));
    config.binary_extensions = merged_bins;

    // Set token mode if tokens specified
    if !config.tokens.is_empty() {
        config.token_mode = true;
    }

    match serialize_repo(&config) {
        Ok((output, files)) => {
            json!({
                "content": [{
                    "type": "text",
                    "text": output
                }],
                "metadata": {
                    "files_count": files.len()
                }
            })
        }
        Err(e) => {
            json!({
                "isError": true,
                "content": [{
                    "type": "text",
                    "text": format!("Error serializing repository: {}", e)
                }]
            })
        }
    }
}

fn main() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                let mut out = stdout.lock();
                serde_json::to_writer(&mut out, &error_response)?;
                out.write_all(b"\n")?;
                out.flush()?;
                continue;
            }
        };

        // Notifications don't get responses
        if request.id.is_none() {
            handle_request(&request);
            continue;
        }

        let result = handle_request(&request);

        let response = if result.get("error").is_some() {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.unwrap_or(Value::Null),
                result: None,
                error: Some(JsonRpcError {
                    code: result["error"]["code"].as_i64().unwrap_or(-32603),
                    message: result["error"]["message"]
                        .as_str()
                        .unwrap_or("Internal error")
                        .to_string(),
                }),
            }
        } else {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.unwrap_or(Value::Null),
                result: Some(result),
                error: None,
            }
        };

        let mut out = stdout.lock();
        serde_json::to_writer(&mut out, &response)?;
        out.write_all(b"\n")?;
        out.flush()?;
    }

    Ok(())
}
