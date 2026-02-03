use serde::{Serialize, Deserialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

#[derive(Serialize, Debug)]
pub struct Chunk {
    pub url: String,
    pub title: String,
    pub chunk_id: usize,
    pub chunk_text: String,
}

// put this in lib later and take it out of main and here
#[derive(Serialize, Deserialize, Debug)]
pub struct Page {
    url: String,
    title: String,
    content: String,
    links_found: usize,
}

pub fn chunk_pages(input_path: &str, output_path: &str, chunk_size: usize) -> std::io::Result<()> {
    let input_file = File::open(input_path)?;
    let reader = BufReader::new(input_file);
    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)?;

    for line in reader.lines() {
        let line = line?;
        let page: Page = match serde_json::from_str(&line) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let words: Vec<&str> = page.content.split_whitespace().collect();
        let mut chunk_id = 0;
        for chunk in words.chunks(chunk_size) {
            let chunk_text: String = chunk.join(" ");
            let chunk_struct = Chunk {
                url: page.url.clone(),
                title: page.title.clone(),
                chunk_id,
                chunk_text,
            };
            let json = serde_json::to_string(&chunk_struct)?;
            writeln!(output_file, "{}", json)?;
            chunk_id += 1;

        }

    }
     Ok(())

}
