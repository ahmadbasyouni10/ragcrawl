use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

#[derive(Deserialize)]
pub struct Chunk {
    pub url: String,
    pub title: String, 
    pub chunk_id: usize,
    pub chunk_text: String,
}

#[derive(Serialize)]
pub struct VectorChunk {
    pub url: String,
    pub title: String,
    pub chunk_id: usize,
    pub chunk_text: String,
    pub embedding: Vec<f32>,
}

#[derive(Serialize)]
struct OpenAIRequest <'a> {
    input: &'a str,
    model: &'a str,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    data: Vec<OpenAIEmbedding>,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
}

pub fn embed_chunks(
    input_path: &str,
    output_path: &str,
    api_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let infile = File::open(input_path)?;
    let reader = BufReader::new(infile);
    let mut outfile = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)?;
    let client = reqwest::blocking::Client::new();
    for line in reader.lines() {
        let line = line?;
        let chunk: Chunk = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let req_body = OpenAIRequest {
            input: &chunk.chunk_text,
            model: "text-embedding-ada-002",
        };

        let resp = client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(api_key)
            .json(&req_body)
            .send()?
            .error_for_status()?;

        let resp_json: OpenAIResponse = resp.json()?;
        let embedding = resp_json.data[0].embedding.clone();

        let vector_chunk = VectorChunk {
            url: chunk.url,
            title: chunk.title,
            chunk_id: chunk.chunk_id,
            chunk_text: chunk.chunk_text,
            embedding,
        };
        let out_json = serde_json::to_string(&vector_chunk)?;
        writeln!(outfile, "{}", out_json)?;
    }
    Ok(())
}