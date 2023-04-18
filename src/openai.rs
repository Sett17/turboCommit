#![allow(dead_code)]

use colored::Colorize;
use crossterm::cursor::{MoveToColumn, MoveToPreviousLine};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, terminal};
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{fmt, process};

use crate::animation;
use crate::util::count_lines;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub const fn system(content: String) -> Self {
        Self {
            role: Role::System,
            content,
        }
    }
    pub const fn user(content: String) -> Self {
        Self {
            role: Role::User,
            content,
        }
    }
    pub const fn assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRoot {
    pub error: Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub message: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({:?}): {:?}",
            self.type_field.red(),
            self.code,
            self.message
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub model: String,
    pub messages: Vec<Message>,
    pub n: i32,
    pub temperature: f64,
    pub frequency_penalty: f64,
    stream: bool,
}

impl Request {
    pub fn new(
        model: String,
        messages: Vec<Message>,
        n: i32,
        temperature: f64,
        frequency_penalty: f64,
    ) -> Self {
        Self {
            model,
            messages,
            n,
            temperature,
            frequency_penalty,
            stream: true,
        }
    }

    pub async fn execute(
        &self,
        api_key: String,
        no_animations: bool,
        model: Model,
        prompt_tokens: usize,
    ) -> anyhow::Result<Vec<String>> {
        let choices = vec![String::new(); self.n as usize];

        let loading_ai_animation =
            animation::start(String::from("Asking AI..."), no_ai_anim, std::io::stdout()).await;

        let json = serde_json::to_string(self)?;

        let request_builder = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .header("Content-Type", "application/json")
            .bearer_auth(api_key)
            .body(json);

        let term_width = terminal::size()?.0 as usize;

        let mut stdout = std::io::stdout();

        let mut es = EventSource::new(request_builder)?;
        let mut lines_to_move_up = 0;
        let mut response_tokens = 0;

        while let Some(event) = es.next().await {
            if no_animations {
                match event {
                    Ok(Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            break;
                        }
                        let resp = serde_json::from_str::<Response>(&message.data)
                            .map_or_else(|_| Response::default(), |r| r);
                        response_tokens += 1;
                        for choice in resp.choices {
                            if let Some(content) = choice.delta.content {
                                choices[choice.index as usize].push_str(&content);
                            }
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                    _ => {}
                }
            } else {
                if !loading_ai_animation.is_finished() {
                    loading_ai_animation.abort();
                    execute!(
                        std::io::stdout(),
                        Clear(ClearType::CurrentLine),
                        MoveToColumn(0),
                    )?;
                    print!("\n\n")
                }
                match event {
                    Ok(Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            break;
                        }
                        execute!(stdout, MoveToPreviousLine(lines_to_move_up),)?;
                        lines_to_move_up = 0;
                        execute!(stdout, Clear(ClearType::FromCursorDown),)?;
                        let resp = serde_json::from_str::<Response>(&message.data)
                            .map_or_else(|_| Response::default(), |r| r);
                        response_tokens += 1;
                        for choice in resp.choices {
                            if let Some(content) = choice.delta.content {
                                choices[choice.index as usize].push_str(&content);
                            }
                        }
                        for (i, choice) in choices.iter().enumerate() {
                            let outp = format!(
                                "{}{}\n{}\n",
                                if i == 0 {
                                    format!(
                                        "This used {} tokens costing you about {}\n",
                                        format!("{}", response_tokens + prompt_tokens).purple(),
                                        format!(
                                            "~${:0.4}",
                                            model.cost(prompt_tokens, response_tokens)
                                        )
                                        .purple()
                                    )
                                    .bright_black()
                                } else {
                                    "".bright_black()
                                },
                                format!("[{}]====================", format!("{i}").purple())
                                    .bright_black(),
                                choice,
                            );
                            print!("{outp}");
                            lines_to_move_up += count_lines(&outp, term_width) - 1;
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                    _ => {}
                }
            }
        }

        if no_animations {
            println!(
                "This used {} tokens costing you about {}\n",
                format!("{}", response_tokens + prompt_tokens).purple(),
                format!("~${:0.4}", model.cost(prompt_tokens, response_tokens)).purple()
            );
            for (i, choice) in choices.iter().enumerate() {
                println!(
                    "[{}]====================\n{}\n",
                    format!("{i}").purple(),
                    choice
                );
            }
        }

        execute!(
            stdout,
            Print(format!("{}\n", "=======================".bright_black())),
        )?;

        Ok(choices)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Choice {
    pub index: i64,
    pub finish_reason: Option<String>,
    pub delta: Delta,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Delta {
    pub role: Option<Role>,
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

pub fn count_token(s: &str) -> anyhow::Result<usize> {
    let bpe = tiktoken_rs::cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(s);
    Ok(tokens.len())
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum Model {
    #[default]
    Gpt35Turbo,
    Gpt4,
    Gpt432k,
}

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gpt-3.5-turbo" => Ok(Self::Gpt35Turbo),
            "gpt-4" => Ok(Self::Gpt4),
            "gpt-4-32k" => Ok(Self::Gpt432k),
            _ => Err(format!("{} is not a valid model", s)),
        }
    }
}

impl ToString for Model {
    fn to_string(&self) -> String {
        match self {
            Self::Gpt35Turbo { .. } => String::from("gpt-3.5-turbo"),
            Self::Gpt4 { .. } => String::from("gpt-4"),
            Self::Gpt432k { .. } => String::from("gpt-4-32k"),
        }
    }
}

impl Serialize for Model {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Model {
    pub fn cost(&self, prompt_tokens: usize, completion_tokens: usize) -> f64 {
        let (prompt_cost, completion_cost) = match self {
            Self::Gpt35Turbo => (0.002, 0.002),
            Self::Gpt4 => (0.03, 0.06),
            Self::Gpt432k => (0.06, 0.12),
        };
        (prompt_tokens as f64).mul_add(
            prompt_cost / 1000.0,
            (completion_tokens as f64) * (completion_cost / 1000.0),
        )
    }
    pub const fn context_size(&self) -> usize {
        match self {
            Self::Gpt35Turbo => 4096,
            Self::Gpt4 => 8192,
            Self::Gpt432k => 32768,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_enum() {
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&Role::Assistant).unwrap(),
            "\"assistant\""
        );
    }

    #[test]
    fn test_message_struct() {
        let system_message = Message::system("Test system message".to_string());
        let user_message = Message::user("Test user message".to_string());
        let assistant_message = Message::assistant("Test assistant message".to_string());

        assert_eq!(system_message.role, Role::System);
        assert_eq!(system_message.content, "Test system message");

        assert_eq!(user_message.role, Role::User);
        assert_eq!(user_message.content, "Test user message");

        assert_eq!(assistant_message.role, Role::Assistant);
        assert_eq!(assistant_message.content, "Test assistant message");
    }

    #[test]
    fn test_request_new() {
        let messages = vec![
            Message::system("Test system message".to_string()),
            Message::user("Test user message".to_string()),
        ];

        let request = Request::new("gpt-3.5-turbo".to_string(), messages.clone(), 1, 0.8, 0.5);

        assert_eq!(request.model, "gpt-3.5-turbo");
        assert_eq!(request.messages, messages);
        assert_eq!(request.n, 1);
        assert_eq!(request.temperature, 0.8);
        assert_eq!(request.frequency_penalty, 0.5);
        assert_eq!(request.stream, true);
    }

    #[test]
    fn test_model_from_str_and_to_string() {
        let gpt35_turbo = Model::from_str("gpt-3.5-turbo").unwrap();
        let gpt4 = Model::from_str("gpt-4").unwrap();
        let gpt432k = Model::from_str("gpt-4-32k").unwrap();

        assert_eq!(gpt35_turbo.to_string(), "gpt-3.5-turbo");
        assert_eq!(gpt4.to_string(), "gpt-4");
        assert_eq!(gpt432k.to_string(), "gpt-4-32k");
    }

    #[test]
    fn test_model_cost() {
        let gpt35_turbo = Model::Gpt35Turbo;
        let gpt4 = Model::Gpt4;
        let gpt432k = Model::Gpt432k;

        assert_eq!(gpt35_turbo.cost(1000, 1000), 0.004);
        assert_eq!(gpt4.cost(1000, 1000), 0.09);
        assert_eq!(gpt432k.cost(1000, 1000), 0.18);
    }

    #[test]
    fn test_model_context_size() {
        let gpt35_turbo = Model::Gpt35Turbo;
        let gpt4 = Model::Gpt4;
        let gpt432k = Model::Gpt432k;

        assert_eq!(gpt35_turbo.context_size(), 4096);
        assert_eq!(gpt4.context_size(), 8192);
        assert_eq!(gpt432k.context_size(), 32768);
    }

    #[test]
    fn test_count_token() {
        let test_string = "Large Language Model";
        let token_count = count_token(test_string).unwrap();
        assert_eq!(token_count, 3);
    }
}
