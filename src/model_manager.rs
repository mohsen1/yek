use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Mutex;
use tiktoken_rs::o200k_base;
use tokenizers::Tokenizer;

lazy_static::lazy_static! {
    static ref MODEL_CACHE: Mutex<HashMap<String, Tokenizer>> = Mutex::new(HashMap::new());
}

pub const SUPPORTED_MODEL_FAMILIES: &[&str] = &[
    "openai",    // All OpenAI models
    "claude",    // All Anthropic Claude models
    "mistral",   // All Mistral models
    "mixtral",   // All Mistral models
    "llama",     // All Meta Llama models
    "deepseek",  // DeepSeek models
    "codellama", // All Meta Llama models
];

fn load_tokenizer(path: &str) -> Result<Tokenizer> {
    Tokenizer::from_file(path).map_err(|e| anyhow!("Failed to load tokenizer from {}: {}", path, e))
}

pub fn tokenize(text: &str, model: &str) -> Result<Vec<u32>> {
    // Handle OpenAI models separately as they use tiktoken
    if model == "openai" {
        let encoding =
            o200k_base().map_err(|e| anyhow!("Failed to get o200k_base encoding: {}", e))?;
        let tokens = encoding.encode_with_special_tokens(text);
        return Ok(tokens.into_iter().map(|t| t as u32).collect());
    }

    let mut cache = MODEL_CACHE
        .lock()
        .map_err(|e| anyhow!("Failed to lock cache: {}", e))?;

    // Load tokenizer if not in cache
    if !cache.contains_key(model) {
        let tokenizer = match model {
            "claude" => load_tokenizer("models/claude-3-opus/tokenizer.json")?,
            "mistral" | "mixtral" => load_tokenizer("models/mistral/tokenizer.json")?,
            "deepseek" => load_tokenizer("models/deepseek/tokenizer.json")?,
            m if m.starts_with("llama") || m.starts_with("codellama") => {
                load_tokenizer("models/llama/tokenizer.json")?
            }
            _ => return Err(anyhow!("Unsupported model: {}", model)),
        };
        cache.insert(model.to_string(), tokenizer);
    }

    // Get tokenizer and encode text
    let tokenizer = cache.get(model).unwrap();
    let encoded = tokenizer
        .encode(text, true)
        .map_err(|e| anyhow!("Failed to encode text: {}", e))?;
    Ok(encoded.get_ids().to_vec())
}

pub fn decode_tokens(tokens: &[u32], model: &str) -> Result<String> {
    // Handle OpenAI models separately
    if model == "openai" {
        let encoding =
            o200k_base().map_err(|e| anyhow!("Failed to get o200k_base encoding: {}", e))?;
        let tokens: Vec<usize> = tokens.iter().map(|&t| t as usize).collect();
        return encoding
            .decode(tokens)
            .map_err(|e| anyhow!("Failed to decode tokens: {}", e));
    }

    // Get tokenizer from cache and decode
    let cache = MODEL_CACHE
        .lock()
        .map_err(|e| anyhow!("Failed to lock cache: {}", e))?;
    let tokenizer = cache
        .get(model)
        .ok_or_else(|| anyhow!("Model not found: {}", model))?;
    let result = tokenizer
        .decode(tokens, true)
        .map_err(|e| anyhow!("Failed to decode tokens: {}", e))?;
    Ok(result)
}

pub fn count_tokens(text: &str, model: &str) -> Result<usize> {
    tokenize(text, model)
        .map(|tokens| tokens.len())
        .map_err(|e: anyhow::Error| anyhow!("Token counting failed: {}", e))
}

pub struct ModelManager {
    model: String,
}

impl ModelManager {
    pub fn new(model: Option<&str>) -> Result<Self> {
        let model = model.unwrap_or("openai").to_string();
        if !SUPPORTED_MODEL_FAMILIES.contains(&model.as_str()) {
            return Err(anyhow!("Unsupported model family: {}", model));
        }
        Ok(Self { model })
    }

    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        count_tokens(text, &self.model)
    }
}
