#![allow(dead_code)]

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

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
