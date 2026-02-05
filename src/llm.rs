use serde::{Serialize, Deserialize};
use reqwest::blocking::Client;

pub fn ask_llm(prompt: &str, api_key: &str) -> Result<String, Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct Message<'a> {
        role: &'a str,
        content: &'a str,
    }
    #[derive(Serialize)]
    struct ChatRequest<'a> {
        model: &'a str,
        messages: Vec<Message<'a>>,
        max_tokens: u16,
        temperature: f32,
    }
    #[derive(Deserialize)]
    struct ChatResponse {
        choices: Vec<Choice>,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: MessageContent,
    }
    #[derive(Deserialize)]
    struct MessageContent {
        content: String,
    }

    let client = Client::new();
    let req_body = ChatRequest {
        model: "gpt-3.5-turbo",
        messages: vec![
            Message { role: "system", content: "You are a helpful assistant." },
            Message { role: "user", content: prompt },
        ],
        max_tokens: 150,
        temperature: 0.7,
    };
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&req_body)
        .send()?
        .error_for_status()?;
    let resp_json: ChatResponse = resp.json()?;
    Ok(resp_json.choices[0].message.content.clone())
}