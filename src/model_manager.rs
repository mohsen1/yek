use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::OnceLock;
use tiktoken_rs::{get_bpe_from_model, o200k_base};
use tokenizers::Tokenizer;

static MODEL_CACHE: OnceLock<HashMap<String, Tokenizer>> = OnceLock::new();

pub const SUPPORTED_MODEL_FAMILIES: &[&str] = &[
    "openai",   // All OpenAI models
    "claude",   // All Anthropic Claude models
    "mistral",  // All Mistral models
    "deepseek", // All DeepSeek models
    "llama",    // All Meta Llama models
];

fn load_tokenizer(path: &str) -> Result<Tokenizer> {
    Tokenizer::from_file(path).map_err(|e| anyhow!("Failed to load tokenizer from {}: {}", path, e))
}

pub fn get_tokenizer(model: &str) -> Result<&'static Tokenizer> {
    let cache = MODEL_CACHE.get_or_init(HashMap::new);

    if !cache.contains_key(model) {
        let tokenizer = match model {
            // OpenAI models use tiktoken instead
            m if m == "openai" => {
                return Err(anyhow!("OpenAI models should use tiktoken-rs instead"));
            }
            // BPE models
            m if m.starts_with("claude") => load_tokenizer("models/claude-3-opus/tokenizer.json")?,
            m if m.starts_with("mistral") || m.starts_with("mixtral") => {
                load_tokenizer("models/mistral/tokenizer.json")?
            }
            m if m.starts_with("llama") || m.starts_with("codellama") => {
                load_tokenizer("models/llama/tokenizer.json")?
            }
            m if m.starts_with("deepseek") => load_tokenizer("models/deepseek/tokenizer.json")?,
            _ => return Err(anyhow!("Unsupported model: {}", model)),
        };

        let mut cache_mut = HashMap::new();
        cache_mut.insert(model.to_string(), tokenizer);
        MODEL_CACHE.set(cache_mut).unwrap();
    }

    Ok(cache.get(model).unwrap())
}

pub fn count_tokens(text: &str, model: &str) -> Result<usize> {
    match model {
        // OpenAI models use o200k_base
        m if m == "openai" => {
            let encoding =
                o200k_base().map_err(|e| anyhow!("Failed to get o200k_base encoding: {}", e))?;
            Ok(encoding.encode_with_special_tokens(text).len())
        }
        // Try Hugging Face tokenizers first, fallback to tiktoken BPE
        _ => {
            match get_tokenizer(model) {
                Ok(tokenizer) => {
                    let encoded = tokenizer
                        .encode(text, true)
                        .map_err(|e| anyhow!("Failed to encode text with HF tokenizer: {}", e))?;
                    Ok(encoded.get_ids().len())
                }
                Err(_) => {
                    // Fallback to tiktoken BPE
                    let encoding = get_bpe_from_model("openai")
                        .map_err(|e| anyhow!("Failed to get tiktoken BPE encoding: {}", e))?;
                    Ok(encoding.encode_with_special_tokens(text).len())
                }
            }
        }
    }
}
