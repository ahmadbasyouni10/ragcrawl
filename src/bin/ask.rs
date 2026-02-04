use ragcrawl::search;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

fn main() {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        panic!("Not enough arguments, usage: cargo run --bin ask <query>");
    }
    let query = &args[1..].join(" ");
    println!("Query: {}", query);
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        println!("Please set the OPENAI_API_KEY environment variable.");
        std::process::exit(1);
    });
    let top_k = 5;
    println!("Retrieving top {} relevant chunks...", top_k);
    let results = search::search(
        query,
        &api_key,
        "vector_chunks.jsonl",
        top_k,
    ).unwrap_or_else(|e| {
        println!("Error during search: {}", e);
        std::process::exit(1);
    });

    println!("\n Retrieved Chunks");
    for (i, chunk) in results.iter().enumerate() {
        println!("Chunk {}:", i + 1);
        println!("Title: {}", chunk.title);
        println!("URL: {}", chunk.url);
        println!("Content: {}\n", chunk.chunk_text);
        println!("\n\n");
    }

    let context = results.iter()
        .map(|chunk| format!("Title: {}\nURL: {}\nContent: {}\n", chunk.title, chunk.url, chunk.chunk_text))
        .collect::<Vec<String>>()
        .join("\n---\n");

    let prompt = format!(
        "Use the following context to answer the question:\n\n{}\n\nQuestion: {}",
        context, query
    );

    println!("Prompt and context sent to LLM");

    let answer = ask_llm(&prompt, &api_key).unwrap_or_else(|e| {
        println!("Error during LLM request: {}", e);
        std::process::exit(1);
    });
    println!("Answer: {}", answer);
}

fn ask_llm(prompt: &str, api_key: &str) -> Result<String, Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct Message<'a> {
        role: &'a str,
        content: &'a str,
    }
    #[derive(Serialize)]
    struct ChatRequest<'a> {
        model: &'a str,
        messages: Vec<Message<'a>>,
        max_tokens: u16,
        temperature: f32,
    }
    #[derive(Deserialize)]
    struct ChatResponse {
        choices: Vec<Choice>,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: MessageContent,
    }
    #[derive(Deserialize)]
    struct MessageContent {
        content: String,
    }

    let client = Client::new();
    let req_body = ChatRequest {
        model: "gpt-3.5-turbo",
        messages: vec![
            Message { role: "system", content: "You are a helpful assistant." },
            Message { role: "user", content: prompt },
        ],
        max_tokens: 150,
        temperature: 0.7,
    };

    let res = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&req_body)
        .send()?
        .error_for_status()?;

    let res_json: ChatResponse = res.json()?;
    Ok(res_json.choices[0].message.content.clone())
}