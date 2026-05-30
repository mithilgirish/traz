use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Mutex, OnceLock};

static EMBEDDER: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

/// Retrieve or initialize the shared fastembed TextEmbedding model.
/// Downloads the AllMiniLML6V2 model to `~/.local/share/traz/models/` if not present.
pub fn get_embedder() -> Result<&'static Mutex<TextEmbedding>> {
    if let Some(emb) = EMBEDDER.get() {
        return Ok(emb);
    }
    let mut cache_dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    cache_dir.push("traz");
    cache_dir.push("models");
    std::fs::create_dir_all(&cache_dir)?;

    let options = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
        .with_cache_dir(cache_dir)
        .with_show_download_progress(false);

    let embedder = TextEmbedding::try_new(options)?;
    let _ = EMBEDDER.set(Mutex::new(embedder));
    Ok(EMBEDDER.get().unwrap())
}

/// Generate a vector embedding for a single string.
pub fn embed_text(text: &str) -> Result<Vec<f32>> {
    let embedder_mutex = get_embedder()?;
    let mut embedder = embedder_mutex.lock().unwrap();
    let embeddings = embedder.embed(vec![text], None)?;
    if let Some(embedding) = embeddings.into_iter().next() {
        Ok(embedding)
    } else {
        anyhow::bail!("Failed to generate embedding: empty result")
    }
}

/// Compute cosine similarity between two vector slices.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a * norm_b)
}

/// Check if the local embedding model files exist under the traz data directory.
pub fn is_embedding_model_downloaded() -> bool {
    let mut cache_dir = match dirs::data_dir() {
        Some(d) => d,
        None => return false,
    };
    cache_dir.push("traz");
    cache_dir.push("models");

    if !cache_dir.exists() {
        return false;
    }

    // Recursively check if any .onnx file exists
    fn has_onnx_file(path: &std::path::Path) -> bool {
        if path.is_file() {
            return path.extension().is_some_and(|ext| ext == "onnx");
        }
        if path.is_dir()
            && let Ok(entries) = std::fs::read_dir(path)
        {
            for entry in entries.flatten() {
                if has_onnx_file(&entry.path()) {
                    return true;
                }
            }
        }
        false
    }

    has_onnx_file(&cache_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-5);
    }
}
