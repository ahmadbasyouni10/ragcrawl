use serde::{Serialize, Deserialize};

pub mod search;
pub mod chunking;
pub mod embedding;
pub mod crawler;
pub mod llm;
pub mod cli;

#[derive(Serialize, Debug)]
pub struct Chunk {
    pub url: String,
    pub title: String,
    pub chunk_id: usize,
    pub chunk_text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Page {
    pub url: String,
    pub title: String,
    pub content: String,
    pub links_found: usize,
}