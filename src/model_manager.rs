use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::OnceLock;
use tiktoken_rs::{get_bpe_from_model, o200k_base};
use tokenizers::Tokenizer;

static MODEL_CACHE: OnceLock<HashMap<String, Tokenizer>> = OnceLock::new();

pub const SUPPORTED_MODELS: &[&str] = &[
    // OpenAI (using tiktoken)
    "gpt-4o",
    "gpt-4o-2024-08-06",
    "chatgpt-4o-latest",
    "gpt-4o-mini",
    "gpt-4o-mini-2024-07-18",
    "o1",
    "o1-2024-12-17",
    "o1-mini",
    "o1-mini-2024-09-12",
    "o1-preview",
    "o1-preview-2024-09-12",
    "gpt-4o-realtime-preview",
    "gpt-4o-realtime-preview-2024-12-17",
    "gpt-4o-mini-realtime-preview",
    "gpt-4o-mini-realtime-preview-2024-12-17",
    "gpt-4o-audio-preview",
    "gpt-4o-audio-preview-2024-12-17",
    // Rest using Hugging Face tokenizers
    // Anthropic Claude 3.5 (BPE)
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet-latest",
    "claude-3-5-haiku-20241022",
    "claude-3-5-haiku-latest",
    // Anthropic Claude 3 (BPE)
    "claude-3-opus-20240229",
    "claude-3-opus-latest",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    // Mistral (BPE)
    "mistral-7b-v0-3",
    "mistral-nemo-12b",
    "mistral-openorca-7b",
    "mistral-large-123b",
    "mistral-small-22b",
    "mistrallite-7b",
    "mixtral-8x7b",
    "mixtral-8x22b",
    // Meta Llama Models (BPE)
    "llama-3-3-70b",
    "llama-3-2-1b",
    "llama-3-2-3b",
    "llama-3-2-vision-11b",
    "llama-3-2-vision-90b",
    "llama-3-1-8b",
    "llama-3-1-70b",
    "llama-3-1-405b",
    "llama-3-8b",
    "llama-3-70b",
    "llama-2-7b",
    "llama-2-13b",
    "llama-2-70b",
    // Code Llama (BPE)
    "codellama-7b",
    "codellama-13b",
    "codellama-34b",
    "codellama-70b",
    // Tiny Llama (BPE)
    "tinyllama-1-1b",
];

fn load_tokenizer(path: &str) -> Result<Tokenizer> {
    Tokenizer::from_file(path).map_err(|e| anyhow!("Failed to load tokenizer from {}: {}", path, e))
}

pub fn get_tokenizer(model: &str) -> Result<&'static Tokenizer> {
    let cache = MODEL_CACHE.get_or_init(HashMap::new);

    if !cache.contains_key(model) {
        let tokenizer = match model {
            // OpenAI models use tiktoken instead
            m if m.starts_with("gpt") || m.starts_with("o1") => {
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
        m if m.starts_with("gpt-4o") || m.starts_with("o1") => {
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
                    let encoding = get_bpe_from_model("gpt-4")
                        .map_err(|e| anyhow!("Failed to get tiktoken BPE encoding: {}", e))?;
                    Ok(encoding.encode_with_special_tokens(text).len())
                }
            }
        }
    }
}
