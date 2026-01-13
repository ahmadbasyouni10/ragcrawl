use scraper::{Html, Selector, ElementRef};
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
    base_domain: String,
}

impl Spider { 
    fn new(start_url: &str) -> Result<Self, String> {
        let url = Url::parse(start_url)
            .map_err(|e| format!("Invalid URL: {}", e))?;

        let domain = url.host_str()
            .ok_or("No domain found")?
            .to_string();

        let base_domain = extract_base_domain(&domain);

        let mut spider = Spider {
            queue: VecDeque::new(),
            visited: HashSet::new(),
            domain,
            base_domain,
        };

        spider.enqueue(start_url.to_string());
        Ok(spider)
    }

    fn enqueue(&mut self, url: String) {
        if !self.is_same_domain(&url) {
            return;
        }

        let normalized = match normalize_url(&url) {
            Ok(n) => n,
            Err(_) => return,
        };

        if !self.visited.contains(&normalized) {
            self.queue.push_back(normalized);
        }
    }

    fn is_same_domain(&self, url: &str) -> bool {
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(host) = parsed_url.host_str() {
                return host == self.domain
                    || host == self.base_domain
                    || host.ends_with(&format!(".{}", self.base_domain));
            }
        }
        false
    }

    fn dequeue(&mut self) -> Option<String> {
        self.queue.pop_front()
    }

    fn mark_visited(&mut self, url: String) {
        if let Ok(normalized) = normalize_url(&url) {
            self.visited.insert(normalized);
        }
    }
}

struct Config {
    start_url: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            println!("Please provide a url");
            println!("Usage: cargo run <start_url>");
            return Err("Please provide a url\nUsage: cargo run <start_url>");
        }
        let start_url = args[1].clone();
        Ok(Config { start_url })
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        std::process::exit(1);
        });
    let start_url = config.start_url;

    let mut spider = match Spider::new(&start_url) {
        Ok(spider) => spider,
        Err(e) => {
            println!("Error initializing spider: {}", e);
            return;
        }
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("pages.jsonl")
        .expect("Unable to open file");
    
    let mut page_count = 0;
    while let Some(current_url) = spider.dequeue() {
        if spider.visited.contains(&current_url) {
            continue;
        }

        println!("\n[{}] Fetching {}", page_count+1, current_url);

        let html_content = match fetch_page(&current_url) {
            Ok(content) => content,
            Err(e) => {
                println!("Skipped {}", e);
                spider.mark_visited(current_url);
                continue;
            }
        };

        println!("Downloaded {} bytes", html_content.len());

        let found_links = find_links(&current_url, &html_content);

        println!("Found {} links on this page", found_links.len());

        for link in &found_links {
            spider.enqueue(link.clone());
        }

        let (title, content) = extract_content(&html_content);
        if title.to_lowercase().contains("not found") ||
            title.to_lowercase().contains("404") ||
            content.to_lowercase().contains("looks like you've taken a wrong turn") {
                println!("Skipped Error Page");
                spider.mark_visited(current_url);
                continue;
            }
        println!("Title: {}", title);
        println!("Content length: {}", content.len());
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
        page_count += 1;

        if spider.visited.len() >= 40 {
            println!("Limit Reached, stopping!");
            break;
        }
    }

    println!("\n Done, Saved to pages.jsonl");
    println!("Total pages crawled: {}", page_count);
}

fn should_skip_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    let skip_patterns = [
        "/ad/", "/ads/", "/advert/", "/banner/",
        "doubleclick", "googlesyndication", "googleadservices",
        "/track/", "/tracking/", "/analytics/", "/pixel", "/beacon",
        
        ".jpg", ".jpeg", ".png", ".gif", ".svg", ".webp", ".ico", ".bmp",
        ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
        ".zip", ".tar", ".gz", ".rar", ".7z",
        ".mp4", ".mp3", ".avi", ".mov", ".wmv", ".flv", ".webm",
        ".wav", ".ogg", ".m4a",
        
        ".css", ".js", ".json", ".xml", ".woff", ".woff2", ".ttf", ".eot",
        
        "/feed/", "/rss/", "/atom/", "/api/", "/graphql/",
        
        "login", "signin", "signup", "register", "auth",
        "cart", "checkout", "account", "profile", "settings",
    ];

    skip_patterns.iter().any(|pattern: &&str| url_lower.contains(pattern))

}

fn fetch_page(target_url: &String) -> Result<String, String> {
    if should_skip_url(target_url) {
        return Err("Skipped URL based on patterns".to_string());
    }
    println!("url: {}", target_url);

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e: reqwest::Error| e.to_string())?;

    let response = client.get(target_url)
        .send()
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP error: {}", status));
    }

    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        let content_type_str = content_type.to_str().unwrap_or("");
        if !content_type_str.starts_with("text/html") {
            return Err(format!("non HTML ({})", content_type_str));
        }
    }

    let text = response.text().map_err(|e| e.to_string())?;
    if text.len() < 1000 {
        return Err(format!("Page too small ({} bytes)", text.len()));
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
                links.push(absolute_url.to_string());
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

    let main_element = document.select(&Selector::parse("main").unwrap())
        .next()
        .or_else(|| document.select(&Selector::parse("article").unwrap()).next())
        .or_else(|| document.select(&Selector::parse("body").unwrap()).next());

    let content = if let Some(element) = main_element {
        extract_text_from_element(element)
    } else {
        String::new()
    };

    let cleaned = clean_whitespace(&content);
    let truncated: String = cleaned.chars().take(5000).collect();
    (title.trim().to_string(), truncated)
}

fn extract_text_from_element(element: ElementRef) -> String {
    let mut text = String::new();

    for node in element.descendants() {
        if let Some(el) = node.value().as_element() {
            let tag = el.name();
            let class = el.attr("class").unwrap_or("");
            let id = el.attr("id").unwrap_or("");

            if should_skip_element(tag, class, id) {
                continue;
            }

        }

        if let Some(text_node) = node.value().as_text() {
            text.push_str(text_node);
            text.push(' ');
        }
    }

    text
}

fn should_skip_element(tag: &str, class: &str, id: &str) -> bool {
    let skip_tags = [ "script", "style", "noscript", "iframe",
    "nav", "footer", "aside", "header",
    "form", "input", "button"];
    let skip_keywords = [
    "nav", "navigation", "menu", "sidebar", "header", "footer",
    "ad", "ads", "advert", "advertisement", "banner", "promo", "sponsored",
    "social", "share", "sharing",
    "comment", "comments", "related", "recommended",
    "cookie", "consent", "subscribe", "login", "signin", "signup", "register",
    "button", "buttons", "clip-button", "play-button",
    "toc", "table-of-contents"];

    if skip_tags.contains(&tag) {
        return true;
    }

    for keyword in &skip_keywords {
        if class.to_lowercase().contains(keyword) || id.to_lowercase().contains(keyword) {
            return true;
        }
    }

    false
}
fn clean_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
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

fn extract_base_domain(domain: &str) -> String { 
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2..].join(".")
    } else {
        domain.to_string()
    }
}