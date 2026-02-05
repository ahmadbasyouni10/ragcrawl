use ragcrawl::{search, llm};

fn main() {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run --bin ask <your question>");
        std::process::exit(1);
    }
    let query = &args[1..].join(" ");
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

    println!("\nRetrieved Chunks");
    for (i, chunk) in results.iter().enumerate() {
        println!("Chunk {}:", i + 1);
        println!("Title: {}", chunk.title);
        println!("URL: {}", chunk.url);
        println!("Content: {}\n", chunk.chunk_text);
    }

    let context = results.iter()
        .map(|chunk| format!("Title: {}\nURL: {}\nContent: {}\n", chunk.title, chunk.url, chunk.chunk_text))
        .collect::<Vec<String>>()
        .join("\n\n");

    let prompt = format!(
        "Use the following context to answer the question:\n\n{}\n\nQuestion: {}",
        context, query
    );

    println!("\nPrompt Sent to LLM\n{}\n", prompt);

    let answer = llm::ask_llm(&prompt, &api_key).unwrap_or_else(|e| {
        println!("Error during LLM request: {}", e);
        std::process::exit(1);
    });
    println!("\nLLM Answer\n{}", answer);
}