use ragcrawl::{crawler, chunking, embedding, search, llm};

fn main() {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run <start_url> <question>");
        std::process::exit(1);
    }
    let start_url = &args[1];
    let question = args[2..].join(" ");
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        println!("Please set the OPENAI_API_KEY environment variable.");
        std::process::exit(1);
    });

    println!("Crawling: {}", start_url);
    crawler::crawl_site_parallel(start_url, 40, "pages.jsonl", 8).expect("Crawling failed");

    println!("Chunking...");
    chunking::chunk_pages("pages.jsonl", "chunks.jsonl", 800).expect("Chunking failed");

    println!("Embedding...");
    embedding::embed_chunks("chunks.jsonl", "vector_chunks.jsonl", &api_key).expect("Embedding failed");

    println!("Searching...");
    let top_k = 5;
    let results = search::search(&question, &api_key, "vector_chunks.jsonl", top_k).expect("Search failed");

    println!("\nRetrieved Chunks");
    for (i, chunk) in results.iter().enumerate() {
        println!("Chunk {}:", i + 1);
        println!("Title: {}", chunk.title);
        println!("URL: {}", chunk.url);
        println!("Content: {}\n", &chunk.chunk_text[..chunk.chunk_text.len().min(300)]);
    }
    let context = results.iter()
        .map(|chunk| format!("Title: {}\nURL: {}\nContent: {}\n", chunk.title, chunk.url, chunk.chunk_text))
        .collect::<Vec<String>>()
        .join("\n\n");
    let prompt = format!(
        "Use the following context to answer the question:\n\n{}\n\nQuestion: {}",
        context, question
    );
    let answer = llm::ask_llm(&prompt, &api_key).unwrap_or_else(|e| {
        println!("Error during LLM request: {}", e);
        std::process::exit(1);
    });
    println!("\nLLM Answer-\n{}", answer);
}