use ragcrawl::search;
fn main() {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 { 
        println!("Usage: cargo run --bin cosine <query>");
        std::process::exit(1);
    }
    let query = &args[1..].join(" ");
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        println!("Please set the OPENAI_API_KEY environment variable.");
        std::process::exit(1);
    });
    let tok_k = 5;
    let results = search::search(
        query,
        &api_key,
        "vector_chunks.jsonl",
        tok_k,
    ).unwrap_or_else(|e| {
        println!("Error during search: {}", e);
        std::process::exit(1);
    });
    for (i, chunk) in results.iter().enumerate() {
        println!("Result {}: {} ({})\n{}\n\n\n", i + 1, chunk.title, chunk.url, chunk.chunk_text);
    }
}