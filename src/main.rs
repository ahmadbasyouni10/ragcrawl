use scraper::{Html, Selector};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Please provide a url");
        return;
    }
    println!("Arguments: {:?}", args);
    println!("{}", args[1]);

    // Use reference to keep ownership of url in main
    let url = &args[1];

    println!("Fetching {}", url);

    let html_text = fetch_page(url);

    println!("Got HTML of size: {}", html_text.len());

    find_links(&html_text);

}

fn fetch_page(target_url: &String) -> String {
    println!("url: {}", target_url);

    let response = reqwest::blocking::get(target_url)
        .expect("Could Not make the request");

    println!("Response: {:?}", response);

    let body = response.text()
        .expect("Could not read response text");

    println!("Sending response.text to main function: {}", body);
    body

}

fn find_links(html_content: &String) {
    println!("HTML CONTENT: {}", html_content);
    let document = Html::parse_document(html_content);
    let selector : Selector = Selector::parse("a").unwrap();
    for element in document.select(&selector) {
        match element.value().attr("href") {
            Some(link) => println!("Found link: {}", link),
            None => println!("Found tag with no link"),
        }
    }

}