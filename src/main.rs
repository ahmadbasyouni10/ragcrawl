use scraper::{Html, Selector};
use std::{env};
use std::collections::{HashSet, VecDeque};
use serde::Serialize;
use std::fs::OpenOptions;
use url::Url;
use std::io::Write;


#[derive(Serialize, Debug)]
struct Page {
    url: String,
    title: String,
    content: String,
    links_found: usize,
}

struct Spider {
    queue: VecDeque<String>,
    visited: HashSet<String>,
    domain: String,
}

impl Spider { 
    fn new(start_url: &str) -> Result<Self, String> {
        let url = Url::parse(start_url)
            .map_err(|e| format!("Invalid URL: {}", e))?;

        let domain = url.host_str()
            .ok_or("No domain found")?
            .to_string();

        let mut spider = Spider {
            queue: VecDeque::new(),
            visited: HashSet::new(),
            domain,
        };
        spider.enqueue(start_url.to_string());
        Ok(spider)
    }

    fn enqueue(&mut self, url: String) {
        if !self.visited.contains(&url) && self.is_same_domain(&url) {
            self.queue.push_back(url);
        }
    }

    fn is_same_domain(&self, url: &str) -> bool {
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(host) = parsed_url.host_str() {
                return host == self.domain;
            }
        }
        false
    }

    fn dequeue(&mut self) -> Option<String> {
        self.queue.pop_front()
    }

    fn mark_visited(&mut self, url: String) {
        self.visited.insert(url);
    }
}
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please provide a url");
        return;
    }

    let start_url = &args[1];

    let mut spider = match Spider::new(start_url) {
        Ok(spider) => spider,
        Err(e) => {
            println!("Error initializing spider: {}", e);
            return;
        }
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("pages.jsonl")
        .expect("Unable to open file");

    while let Some(current_url) = spider.dequeue() {
        if spider.visited.contains(&current_url) {
            continue;
        }

        println!("Fetching {}", current_url);

        let html_content = fetch_page(&current_url);

        println!("Got HTML of size: {}", html_content.len());

        let found_links= find_links(&html_content);

        println!("Found {} links on this page", found_links.len());

        for link in found_links {
            if link.starts_with("http") {
                spider.enqueue(link);
            }
        }

        spider.mark_visited(current_url);

        if spider.visited.len() >= 5 {
            println!("Limit Reached, stopping!");
            break;
        }
    }
}

fn fetch_page(target_url: &String) -> String {
    println!("url: {}", target_url);

    let response = match reqwest::blocking::get(target_url) {
        Ok(resp) => resp,
        Err(_) => return String::new(),
    };

    match response.text() {
        Ok(text) => text,
        Err(_) => return String::new(),
    }

}

fn find_links(html_content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let document = Html::parse_document(html_content);
    let selector : Selector = Selector::parse("a").unwrap();
    for element in document.select(&selector) {
        match element.value().attr("href") {
            Some(link) => links.push(link.to_string()),
            None => println!("Found tag with no link"),
        }
    }
    links
}