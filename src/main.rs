#![allow(dead_code)]
// use std::io;
use rustyline::{Cmd, Event, EventHandler, KeyEvent};
use std::env;
use std::fs::OpenOptions;
use std::io::Write; //::{Editor};

use clap::Parser;
use reqwest::blocking::ClientBuilder;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// The model to use
    #[arg(long)]
    model: Option<String>,

    /// Maximum tokens to return
    #[arg(long, default_value_t = 2_000)]
    max_tokens: u32,

    /// The secret key
    #[arg(long)]
    api_key: Option<String>,
}

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
    max_tokens: u32,
}

impl RequestInfo {
    fn new(prompt: String, model: String, temperature: f32, max_tokens: u32) -> Self {
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

#[derive(Debug, Deserialize)]
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
    // Get the command line options
    let cmd_line_opts = Arguments::parse();

    let mut options = OpenOptions::new();
    let mut history_file: File = options
        .write(true)
        .append(true)
        .create(true)
        .open("reply.txt")
        .unwrap();

    let binding: String;
    let api_key = match cmd_line_opts.api_key.as_deref() {
        Some(key) => key,
        None => {
            binding = env::var("OPENAI_API_KEY").unwrap();
            binding.as_str()
        }
    };
    let binding = "text-davinci-003".to_string();
    let model = match cmd_line_opts.model.as_deref() {
        Some(model) => model,
        None => &binding,
    };
    let tokens = cmd_line_opts.max_tokens;

    // Set up readline/rustyline
    // https://github.com/kkawakam/rustyline
    let mut rl = rustyline::Editor::<()>::new().unwrap();

    // Set control keys to control append
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent::ctrl('q')]),
        EventHandler::Simple(Cmd::Interrupt),
    );

    let mut quit: bool = false;

    let prompt: String = "Hello.  Are you ready to answer questions?".to_string();
    let temperature = 0.9;
    let mut request_info =
        RequestInfo::new(prompt.to_string(), model.to_string(), temperature, tokens);
    let client = ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    let mut json: RequestInfo;

    loop {
        _ = history_file
            .write(format!("Q: {}\n", request_info.prompt).as_bytes())
            .unwrap();
        let response = match client
            .post("https://api.openai.com/v1/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&request_info)
            .send()
        {
            Ok(response) => response,
            Err(err) => panic!("{err}"),
        };
        match response.status() {
            StatusCode::OK => println!("success!"),
            s => {
                panic!(
                    "Failed: Status: {}. Response.path({})",
                    s.canonical_reason().unwrap_or("Unknown Reason"),
                    response.url().path(),
                );
            }
        };
        json = response.json().unwrap();
        if json.choices[0].text.is_empty() {
            break;
        }

        _ = history_file
            .write(format!("A: {}\n", json.choices[0].text.trim_start()).as_bytes())
            .unwrap();

        for s in json.choices[0].text.as_str().split_terminator('\n') {
            println!("{}", justify_string(s));
        }
        let readline = rl.readline(">> ");
        let input = match readline {
            Ok(line) => line,
            Err(_) => {
                quit = true;
                "".to_string()
            }
        };

        if quit {
            break;
        }
        println!("You entered: {}", input);
        request_info.prompt = input;
    }
    if !json.choices.is_empty() {
        request_info.prompt = json.choices[0].text.clone();
    }
}
