use ort::session::Session;
use std::path::Path;
use std::sync::Mutex;

use crate::types::EMBEDDING_DIM;

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("ort error: {0}")]
    Ort(String),
    #[error("model not found at {0}")]
    ModelNotFound(String),
    #[error("unexpected output shape")]
    BadShape,
    #[error("lock poisoned")]
    Lock,
}

/// Embeds text using all-MiniLM-L6-v2 via ONNX Runtime.
pub struct Embedder {
    session: Mutex<Session>,
}

impl Embedder {
    pub fn new(model_path: &Path) -> Result<Self, EmbeddingError> {
        if !model_path.exists() {
            return Err(EmbeddingError::ModelNotFound(model_path.display().to_string()));
        }
        let mut builder = Session::builder().map_err(|e| EmbeddingError::Ort(e.to_string()))?;
        builder = builder.with_intra_threads(1).map_err(|e| EmbeddingError::Ort(e.to_string()))?;
        let session = builder.commit_from_file(model_path).map_err(|e: ort::Error| EmbeddingError::Ort(e.to_string()))?;
        Ok(Self { session: Mutex::new(session) })
    }

    /// Embed a single text, returning a normalized 384-d vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let tokens = tokenize(text);
        let seq_len = tokens.len();

        let input_ids: Vec<i64> = tokens.iter().map(|&t| t as i64).collect();
        let attention_mask: Vec<i64> = vec![1i64; seq_len];
        let token_type_ids: Vec<i64> = vec![0i64; seq_len];

        use ort::value::Tensor;
        let t_ids = Tensor::from_array(([1usize, seq_len], input_ids)).map_err(|e| EmbeddingError::Ort(e.to_string()))?;
        let t_mask = Tensor::from_array(([1usize, seq_len], attention_mask)).map_err(|e| EmbeddingError::Ort(e.to_string()))?;
        let t_types = Tensor::from_array(([1usize, seq_len], token_type_ids)).map_err(|e| EmbeddingError::Ort(e.to_string()))?;

        let mut session = self.session.lock().map_err(|_| EmbeddingError::Lock)?;
        let outputs = session.run(ort::inputs![t_ids, t_mask, t_types]).map_err(|e| EmbeddingError::Ort(e.to_string()))?;

        // Output: try_extract_tensor returns (&Shape, &[f32])
        let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(|e| EmbeddingError::Ort(e.to_string()))?;

        // Shape should be [1, seq_len, 384]
        if shape.len() != 3 || shape[2] != EMBEDDING_DIM as i64 {
            return Err(EmbeddingError::BadShape);
        }
        let seq = shape[1] as usize;

        // Mean pooling over sequence dimension
        let mut embedding = vec![0.0f32; EMBEDDING_DIM];
        for i in 0..seq {
            let offset = i * EMBEDDING_DIM;
            for j in 0..EMBEDDING_DIM {
                embedding[j] += data[offset + j];
            }
        }
        let seq_f = seq as f32;
        for v in &mut embedding {
            *v /= seq_f;
        }

        // L2 normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut embedding {
                *v /= norm;
            }
        }

        Ok(embedding)
    }
}

/// Cosine similarity between two normalized vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Minimal whitespace tokenizer producing pseudo-token IDs.
/// TODO: Replace with proper WordPiece tokenizer using the model's vocabulary.
fn tokenize(text: &str) -> Vec<u32> {
    let mut tokens = vec![101u32]; // [CLS]
    for word in text.split_whitespace() {
        let hash = word.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        tokens.push(hash % 30000 + 1000);
    }
    tokens.push(102); // [SEP]
    tokens
}
