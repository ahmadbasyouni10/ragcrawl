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
        let normalized = match normalize_url(&url) {
            Ok(n) => n,
            Err(_) => return,
        };

        if !self.visited.contains(&normalized) && self.is_same_domain(&normalized) {
            self.queue.push_back(normalized);
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

        let html_content = match fetch_page(&current_url) {
            Ok(content) => content,
            Err(e) => {
                println!("Error fetching page: {}", e);
                spider.mark_visited(current_url);
                continue;
            }
        };

        println!("Got HTML of size: {}", html_content.len());

        let found_links = find_links(&current_url, &html_content);

        println!("Found {} links on this page", found_links.len());

        for link in &found_links {
            spider.enqueue(link.clone());
        }

        let (title, content) = extract_content(&html_content);
        let page = Page {
            url: current_url.clone(),
            title,
            content,
            links_found: found_links.len(),
        };

        if let Ok(json) = serde_json::to_string(&page) {
            writeln!(file, "{}", json).ok();
        }

        spider.mark_visited(current_url);

        if spider.visited.len() >= 5 {
            println!("Limit Reached, stopping!");
            break;
        }
    }
}

fn fetch_page(target_url: &String) -> Result<String, String> {
    println!("url: {}", target_url);

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(target_url).send()
        .map_err(|e| e.to_string())?;

    let text = response.text().map_err(|e| e.to_string())?;
    if text.len() < 1000 {
        return Err("Page too small".to_string());
    }
    Ok(text)
}

fn find_links(base_url: &str, html_content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let document = Html::parse_document(html_content);
    let selector : Selector = Selector::parse("a").unwrap();
    let base = match Url::parse(base_url) {
        Ok(url) => url,
        Err(_) => return links,
    };

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(absolute_url) = base.join(href) {
                let scheme = absolute_url.scheme();
                if scheme != "http" && scheme != "https" {
                    continue;
                }
                if let Ok(normalized) = normalize_url(&absolute_url.to_string()) {
                    links.push(normalized);
                }
            }
        }
    }
    links 
}

fn extract_content(html: &str) -> (String, String) {
    let document = Html::parse_document(html);
    let title = document.select(&Selector::parse("title").unwrap())
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or("No Title".to_string());

    let content = document.select(&Selector::parse("body").unwrap())
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join(" "))
        .unwrap_or(String::new());
    (title, content.chars().take(5000).collect())
}

fn normalize_url(url_str: &str) -> Result<String, String> {
    let mut url = Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;

    url.set_fragment(None);

    let path = url.path().to_string();
    if path != "/" && path.ends_with('/') {
        let trimmed = path.trim_end_matches('/');
        url.set_path(trimmed);
    }

    Ok(url.to_string())
}