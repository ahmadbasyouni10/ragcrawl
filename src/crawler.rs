use scraper::{Html, Selector, ElementRef};
use std::collections::{HashSet, VecDeque};
use std::fs::OpenOptions;
use std::sync::{Arc, Mutex};
use url::Url;
use std::io::Write;
use crate::Page;

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

pub fn crawl_site_parallel(start_url: &str, max_pages: usize, output_path: &str, num_workers: usize) -> Result<usize, String> {
    let queue = Arc::new(Mutex::new(VecDeque::from([start_url.to_string()])));
    let visited = Arc::new(Mutex::new(HashSet::new()));
    let file = Arc::new(Mutex::new(OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)
        .map_err(|e| format!("Unable to open file: {}", e))?,
    ));

    let mut handles = Vec::new();
    for _ in 0..num_workers {
        let queue = Arc::clone(&queue);
        let visited = Arc::clone(&visited);
        let file = Arc::clone(&file);

        let handle = std::thread::spawn(move || {
            loop {
                let url = {
                    let mut q = queue.lock().unwrap();
                    q.pop_front()
                };
                let url = match url {
                    Some(u) => u,
                    None => break,
                };

                {
                    let mut v = visited.lock().unwrap();
                    if v.contains(&url) {
                        continue;
                    }
                    v.insert(url.clone());
                    if v.len() >= max_pages {
                        break;
                    }
                }

                let html_content = match fetch_page(&url) {
                    Ok(content) => content,
                    Err(e) => continue,
                };

                let found_links = find_links(&url, &html_content);
                for link in &found_links {
                    let mut q = queue.lock().unwrap();
                    q.push_back(link.clone());
                }
                let (title, content) = extract_content(&html_content);
                if title.to_lowercase().contains("404") || title.to_lowercase().contains("not found") || content.to_lowercase().contains("looks like you've taken a wrong turn") {
                    println!("Skipping Error page: {}", url);
                    continue;
                }
                let page = Page {
                    url: url.clone(),
                    title,
                    content,
                    links_found: found_links.len(),
                };
                let json = serde_json::to_string(&page).unwrap();
                let mut f = file.lock().unwrap();
                writeln!(f, "{}", json).ok();   

            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let v = visited.lock().unwrap();
    println!("Total pages crawled: {}", v.len());
    Ok(v.len())
}