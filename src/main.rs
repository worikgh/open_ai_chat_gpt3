#![allow(dead_code)]
// use std::io;
// TODO:  Make time out a parameter.  Report time out in "> p".
use clap::Parser;
use reqwest::blocking::ClientBuilder;
use reqwest::StatusCode;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::Validator;
use rustyline::{Cmd, CompletionType, Config, EditMode, Editor, Event, EventHandler, KeyEvent};
use rustyline::{Completer, Helper, Hinter};
use serde::{Deserialize, Serialize};
use std::borrow::Cow::{self, Borrowed, Owned};
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write; //::{Editor};
mod get_models;
mod model_example_data;
#[cfg(test)]
use model_example_data::ModelExampleData;
/// `MyHelper` is copied from the examples in `RustyLine` crate
#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

/// `MyHelper` is copied from the examples in `RustyLine` crate
impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// The model to use
    #[arg(long)]
    model: Option<String>,

    /// Maximum tokens to return
    #[arg(long, default_value_t = 2_000)]
    max_tokens: u32,

    /// Temperature for the model.
    #[arg(long, default_value_t = 0.9)]
    temperature: f32,

    /// The secret key
    #[arg(long)]
    api_key: Option<String>,

    #[arg(long)]
    start_prompt: Option<String>,
}

/// Response for a completions request.  See
/// https://platform.openai.com/docs/api-reference/completions/create
#[derive(Debug, Serialize, Deserialize)]
struct CompletionRequestInfo {
    #[serde(skip_serializing)]
    id: String,
    #[serde(skip_serializing)]
    object: String,
    #[serde(skip_serializing)]
    choices: Vec<Choice>,
    #[serde(skip_deserializing)]
    prompt: String,
    model: String,
    #[serde(skip_deserializing)]
    temperature: f32,
    #[serde(skip_deserializing)]
    max_tokens: u32,
}
impl CompletionRequestInfo {
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

/// Response for a "models" query
// {
//   "data": [
//     {
//       "id": "model-id-0",
//       "object": "model",
//       "owned_by": "organization-owner",
//       "permission": [...]
//     },
//     :
//     :
//     :
//   ],
//   "object": "list"
// }
#[derive(Debug, Serialize, Deserialize)]
struct ModelData {
    id: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct ModelRequestInfo {
    data: Vec<ModelData>,
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

fn main() -> rustyline::Result<()> {
    // Get the command line options
    let default_model = "text-davinci-003".to_string();
    let cmd_line_opts = Arguments::parse();
    let mut options = OpenOptions::new();
    let mut conversation_record_file: File = options
        .write(true)
        .append(true)
        .create(true)
        .open("reply.txt")
        .unwrap();

    let _key_binding: String;
    let api_key = match cmd_line_opts.api_key.as_deref() {
        Some(key) => key,
        None => {
            _key_binding = env::var("OPENAI_API_KEY").unwrap();
            _key_binding.as_str()
        }
    };

    // The initialisation prompt, passed to the OpenAI chat-bot
    let _prompt_binding: String;
    let initial_prompt = cmd_line_opts
        .start_prompt
        .as_deref()
        .unwrap_or("Hello.  Are you ready to answer questions?");

    let model = match cmd_line_opts.model.as_deref() {
        Some(model) => model,
        None => &default_model,
    };
    let tokens: u32 = cmd_line_opts.max_tokens;
    let temperature: f32 = cmd_line_opts.temperature;

    // Set up readline/rustyline.  Copied from Rustyline examples
    // https://github.com/kkawakam/rustyline
    env_logger::init();
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();
    let h = MyHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
    rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    // Set control key C-q to quit.  Not really needed.  C-c does this
    // auto-magically
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent::ctrl('q')]),
        EventHandler::Simple(Cmd::Interrupt),
    );

    // Set this to true to exit the min loop
    let mut quit: bool = false;

    let mut request_info = CompletionRequestInfo::new(
        initial_prompt.to_string(),
        model.to_string(),
        temperature,
        tokens,
    );

    // The API client. `reqwest`
    let client = ClientBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();
    let mut json: CompletionRequestInfo;

    let mut count = 1;
    loop {
        _ = conversation_record_file
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

        _ = conversation_record_file
            .write(format!("A: {}\n", json.choices[0].text.trim_start()).as_bytes())
            .unwrap();

        for s in json.choices[0].text.as_str().split_terminator('\n') {
            println!("{}", justify_string(s));
        }
        let mut input: String;

        // Loop around reading the input.
        loop {
            let p = format!("{count}> ");
            rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{p}\x1b[0m");
            let readline = rl.readline(&p);
            input = match readline {
                Ok(line) => line,
                Err(_) => {
                    quit = true;
                    "".to_string()
                }
            };
            count += 1;

            // Check if the input is an instruction.  If the first
            // character is a '>'...
            if input.starts_with("> ") {
                let mut meta = input.split_whitespace();
                // The first word is: ">"
                // The rest of the words are commands for the programme to interpret.
                if let Some(cmd) = meta.nth(1) {
                    // Handle commands here
                    match cmd {
                        "p" => {
                            // Display the parameters
                            println!("Temperature: {temperature}");
                            println!("Model: {model}");
                            println!("Tokens: {tokens}")
                        }
                        "md" => {
                            // Display known models
                            let response: reqwest::blocking::Response = match client
                                .post("https://api.openai.com/v1/models")
                                .header("Content-Type", "application/json")
                                .header("Authorization", format!("Bearer {api_key}"))
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
                            println!("{:?}", &response);
                            json = response.json().unwrap();

                            // let model_json: String = response.json().unwrap();
                            // println!("{model_json}");
                            // curl https://api.openai.com/v1/models \
                            //   -H 'Authorization: Bearer YOUR_API_KEY'
                        }

                        _ => (),
                    };
                }
                continue;
            }
            break;
        }
        if quit {
            break;
        }
        rl.add_history_entry(input.as_str())?;
        println!("You entered: {}", input);
        request_info.prompt = input;
    }
    if !json.choices.is_empty() {
        request_info.prompt = json.choices[0].text.clone();
    }
    rl.append_history("history.txt")
    // Ok(())
}
