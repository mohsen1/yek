use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokenizers::Tokenizer;

static MODEL_CACHE: OnceLock<HashMap<String, Tokenizer>> = OnceLock::new();

pub const SUPPORTED_MODELS: &[&str] = &[
    // OpenAI
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
    // Anthropic Claude 3.5
    "claude-3-5-sonnet-20241022",
    "claude-3-5-sonnet-latest",
    "claude-3-5-haiku-20241022",
    "claude-3-5-haiku-latest",
    // Anthropic Claude 3
    "claude-3-opus-20240229",
    "claude-3-opus-latest",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    // DeepSeek
    "deepseek-chat",
    "deepseek-coder",
    "deepseek-reasoner",
    // Microsoft
    "phi-3",
    // Mistral
    "mistral-8x22b",
    // Meta
    "llama-3-70b",
];

fn load_tokenizer(path: &str) -> Result<Tokenizer> {
    Tokenizer::from_file(path).map_err(|e| anyhow!("Failed to load tokenizer from {}: {}", path, e))
}

pub fn get_tokenizer(model: &str) -> Result<&'static Tokenizer> {
    let cache = MODEL_CACHE.get_or_init(|| HashMap::new());

    if !cache.contains_key(model) {
        let tokenizer = match model {
            m if m.starts_with("claude-3") || m.starts_with("claude-3-5") => {
                load_tokenizer("models/claude-3-opus/tokenizer.json")?
            }
            m if m.starts_with("deepseek") => {
                load_tokenizer("models/deepseek-chat/tokenizer.json")?
            }
            "llama-3-70b" => load_tokenizer("models/llama-3-70b/tokenizer.json")?,
            "mistral-8x22b" => load_tokenizer("models/mistral-8x22b/tokenizer.json")?,
            "phi-3" => load_tokenizer("models/phi-3/tokenizer.json")?,
            m if m.starts_with("gpt") || m.starts_with("o1") => {
                return Err(anyhow!("OpenAI models should use tiktoken-rs instead"));
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
    let count = match model {
        m if m.starts_with("gpt") || m.starts_with("o1") => {
            use tiktoken_rs::get_bpe_from_model;
            let encoding = get_bpe_from_model(m)
                .or_else(|_| get_bpe_from_model("gpt-4o"))
                .map_err(|e| anyhow!("Failed to get tiktoken encoding: {}", e))?;
            encoding.encode_with_special_tokens(text).len()
        }
        _ => {
            let tokenizer = get_tokenizer(model)?;
            let encoded = tokenizer
                .encode(text, true)
                .map_err(|e| anyhow!("Failed to encode text: {}", e))?;
            encoded.get_ids().len()
        }
    };

    Ok(count)
}
