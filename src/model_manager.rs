use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::OnceLock;
use tiktoken_rs::{get_bpe_from_model, o200k_base};
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
    "phi3",
    "phi4",
    // Mistral
    "mistral-7b-v0-3",
    "mistral-nemo-12b",
    "mistral-openorca-7b",
    "mistral-large-123b",
    "mistral-small-22b",
    "mistrallite-7b",
    "mixtral-8x7b",
    "mixtral-8x22b",
    // Meta Llama 3.3
    "llama-3-3-70b",
    // Meta Llama 3.2
    "llama-3-2-1b",
    "llama-3-2-3b",
    "llama-3-2-vision-11b",
    "llama-3-2-vision-90b",
    // Meta Llama 3.1
    "llama-3-1-8b",
    "llama-3-1-70b",
    "llama-3-1-405b",
    // Meta Llama 3
    "llama-3-8b",
    "llama-3-70b",
    // Meta Llama 2
    "llama-2-7b",
    "llama-2-13b",
    "llama-2-70b",
    // Code Llama
    "codellama-7b",
    "codellama-13b",
    "codellama-34b",
    "codellama-70b",
    // Tiny Llama
    "tinyllama-1-1b",
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
    let encoding = match model {
        // GPT-4o and o1 models use o200k_base
        m if m.starts_with("gpt-4o") || m.starts_with("o1") => {
            o200k_base().map_err(|e| anyhow!("Failed to get o200k_base encoding: {}", e))?
        }
        // For other models, use a default encoding
        _ => get_bpe_from_model("gpt-4")
            .map_err(|e| anyhow!("Failed to get tiktoken encoding: {}", e))?,
    };

    Ok(encoding.encode_with_special_tokens(text).len())
}
