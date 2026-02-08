use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use crate::Page;
use crate::Chunk;

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
