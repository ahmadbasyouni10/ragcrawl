use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::error::Error;

#[derive(Deserialize)]
pub struct VectorChunk {
    pub url: String,
    pub title: String,
    pub chunk_id: usize,
    pub chunk_text: String,
    pub embedding: Vec<f32>,
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x*y).sum();
    let norm_a: f32 = a.iter().map(|x| x*x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x*x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

pub fn embed_query(query: &str, api_key: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    #[derive(serde::Serialize)]
    struct OpenAIRequest<'a> {
        input: &'a str,
        model: &'a str,
    }

    #[derive(serde::Deserialize)]
    struct OpenAIResponse {
        data: Vec<OpenAIEmbedding>,
    }

    #[derive(serde::Deserialize)]
    struct OpenAIEmbedding {
        embedding: Vec<f32>,
    }

    let client = reqwest::blocking::Client::new();
    let req_body = OpenAIRequest {
        input: query,
        model: "text-embedding-3-small",
    };

    let res = client.post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&req_body)
        .send()?
        .error_for_status()?;

    let res_json: OpenAIResponse = res.json()?;
    Ok(res_json.data[0].embedding.clone())
}

pub fn search (
    query: &str,
    api_key: &str, 
    vector_path: &str,
    top_k: usize,
) -> Result<Vec<VectorChunk>, Box<dyn Error>> {
    let query_embedding = embed_query(query, api_key)?;
    let file = File::open(vector_path)?;
    let reader = BufReader::new(file);

    let mut scored_chunks = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let chunk: VectorChunk = serde_json::from_str(&line)?;  
        let score = cosine_similarity(&query_embedding, &chunk.embedding);
        scored_chunks.push((chunk, score));
    }
    scored_chunks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    Ok(scored_chunks.into_iter().take(top_k).map(|(chunk, _score)| chunk).collect())
}