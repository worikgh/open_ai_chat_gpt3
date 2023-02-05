#![allow(dead_code)]
use std::io;
use std::io::Write;
use std::fs::OpenOptions;

use reqwest::blocking::ClientBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct RequestInfo {
    #[serde(skip_serializing)]
    choices: Vec<Choice>,
    #[serde(skip_serializing)]
    id: String,
    #[serde(skip_deserializing)]
    prompt: String,
    model: String,
    #[serde(skip_serializing)]
    object: String,
    #[serde(skip_deserializing)]
    temperature: f32,
    #[serde(skip_deserializing)]
    max_tokens: i32,
}

impl RequestInfo {
    fn new(prompt: String, model: String, temperature: f32, max_tokens: i32) -> Self {
        Self {
	    choices: Vec::new(),
	    id: String::new(),
	    object: String::new(),
            prompt,
            model,
            temperature,
            max_tokens,
        }
    }
}

#[derive(Debug,  Deserialize)]
struct Choice {
    text: String,
    logprobs: Option<Vec<f32>>,
    finish_reason: String,
    index: i32,
}

fn justify_string(s: &str) -> String {
    let mut result = String::new();
    let mut line_length = 0;
    let mut words = s.split_whitespace();

    while let Some(word) = words.next() {
        let word_length = word.len();

        if line_length + word_length + 1 > 80 {
            result.push('\n');
            line_length = 0;
        } else if line_length > 0 {
            result.push(' ');
            line_length += 1;
        }

        result.push_str(word);
        line_length += word_length;
    }

    result
}


fn main() {
    let mut options = OpenOptions::new();
    let mut file = options.write(true).append(true).create(true).open("reply.txt").unwrap();
    let api_key = "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";

    let prompt:String  = "Hello.  Are you ready to answer questions?".to_string();
    let model = "text-davinci-003";
    let temperature = 0.9;
    let tokens = 2_500;
    let mut request_info = RequestInfo::new(prompt.to_string(), model.to_string(), temperature, tokens);
    let client = ClientBuilder::new().timeout(std::time::Duration::from_secs(60)).build().unwrap();
    let mut json: RequestInfo;
    loop {
	let response = client
            .post("https://api.openai.com/v1/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&request_info)
            .send()
            .unwrap();
	json = response.json().unwrap();
	if json.choices[0].text.is_empty() {
	    break;
	}
	file.write(&json.choices[0].text.as_bytes()).unwrap();
	// println!("json:>{:?}",json);
	for s in json.choices[0].text.as_str().split_terminator('\n'){
	    println!("{}", justify_string(s));
	}
	// println!("json.choices[0].finish_reason:.{:?}", json.choices[0].finish_reason);
	let mut input = String::new();
	io::stdin().read_line(&mut input).unwrap();
	request_info.prompt = format!("{input}");
	println!("You entered: {}", input);
    }
    if !json.choices.is_empty() {
        request_info.prompt = json.choices[0].text.clone();
    }
}
