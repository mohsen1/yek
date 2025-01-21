#!/bin/bash
set -euo pipefail

# Supported models and their Hugging Face paths
declare -A MODEL_URLS=(
    # Anthropic Claude 3.5
    ["claude-3-5-sonnet-20241022"]="anthropic/claude-3-5-sonnet"
    ["claude-3-5-sonnet-latest"]="anthropic/claude-3-5-sonnet"
    ["claude-3-5-haiku-20241022"]="anthropic/claude-3-5-haiku"
    ["claude-3-5-haiku-latest"]="anthropic/claude-3-5-haiku"
    # Anthropic Claude 3
    ["claude-3-opus-20240229"]="anthropic/claude-3-opus"
    ["claude-3-opus-latest"]="anthropic/claude-3-opus"
    ["claude-3-sonnet-20240229"]="anthropic/claude-3-sonnet"
    ["claude-3-haiku-20240307"]="anthropic/claude-3-haiku"
    # DeepSeek
    ["deepseek-chat"]="deepseek-ai/deepseek-tokenizer"
    ["deepseek-coder"]="deepseek-ai/deepseek-tokenizer"
    # Meta
    ["llama-3-70b"]="meta-llama/Meta-Llama-3-70B"
    # Mistral
    ["mistral-8x22b"]="mistralai/Mistral-8x22B-v0.1"
    # Microsoft
    ["phi-3"]="microsoft/phi-3"
)

# Base directories
MODEL_DIR="./models"
CACHE_DIR="./.cache"
TOKENIZER_FILES=(
    "tokenizer.json" "tokenizer_config.json"
    "special_tokens_map.json" "vocab.json"
    "merges.txt" "vocab.txt" "tokenizer.model"
)

mkdir -p "$MODEL_DIR"
mkdir -p "$CACHE_DIR"

download_model() {
    local model_name=$1
    local repo_path=${MODEL_URLS[$model_name]}
    local target_dir="$MODEL_DIR/$model_name"

    echo "Downloading $model_name tokenizer..."

    mkdir -p "$target_dir"

    # Get file list from Hugging Face API
    local api_url="https://huggingface.co/api/models/$repo_path/tree/main"
    local cache_file="$CACHE_DIR/${model_name}_files.json"

    if [ ! -f "$cache_file" ]; then
        curl -sL "$api_url" -o "$cache_file"
    fi

    # Parse and download required files
    jq -r '.[] | select(.type == "file") | .path' "$cache_file" | while read -r file_path; do
        if [[ " ${TOKENIZER_FILES[@]} " =~ " $(basename "$file_path") " ]]; then
            local dest_path="$target_dir/$file_path"
            mkdir -p "$(dirname "$dest_path")"

            if [ ! -f "$dest_path" ]; then
                echo "Downloading $file_path..."
                curl -sL "https://huggingface.co/$repo_path/resolve/main/$file_path" \
                    -o "$dest_path"
            else
                echo "Skipping existing $file_path"
            fi
        fi
    done
}

# Download all models
for model in "${!MODEL_URLS[@]}"; do
    download_model "$model"
done

echo "All tokenizers downloaded to $MODEL_DIR"
