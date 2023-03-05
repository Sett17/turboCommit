use std::cmp::Ordering;
use std::env;
use std::process::Command;

use colored::Colorize;
use inquire::{Confirm, MultiSelect};
use tiktoken_rs::tiktoken::cl100k_base;
use tokenizers::{InputSequence, Tokenizer};

use openai::{Message};

mod openai;

const SYSTEM_MSG: &str = "You are now an AI that writes conventional commits. The user will give you input in the form of a git diff of all the stages files. Focus more on the why then the what. You shall only answer with the commit message in this format:
<type>[optional scope]: <description>

[optional body]
If the change is a breaking change, put a ! before the :";
const MODEL: &str = "gpt-3.5-turbo";

fn main() {
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(api_key) => api_key,
        Err(_) => {
            println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
            return;
        }
    };

    println!();
    let full_diff = match git_diff() {
        Ok(diff) => diff,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let diff = check_diff(full_diff);

    let mut messages = Vec::new();
    messages.push(Message::system(SYSTEM_MSG));
    messages.push(Message::user(diff));

    let req = openai::Request::new(MODEL, messages);

    let json = serde_json::to_string(&req).unwrap();

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .body(json)
        .send();

    match res {
        Ok(res) => {
            match res.status() {
                reqwest::StatusCode::OK => {
                    let body = res.text().unwrap();
                    let resp = serde_json::from_str::<openai::Response>(&body).unwrap();
                    println!("This used {} token, costing you ~{}$", format!("{}", resp.usage.total_tokens).green(), format!("{}", cost(resp.usage.total_tokens)).green());
                    for choice in resp.choices {
                        println!("===============================");
                        println!("{}", choice.message.content);
                        println!("===============================");
                    }
                }
                _ => {
                    let e = res.text().unwrap();
                    let error = serde_json::from_str::<openai::ErrorRoot>(&e).unwrap().error;
                    println!("{}", error);
                    return;
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            return;
        }
    }
}

fn check_diff<S: Into<String>>(s: S) -> String {
    let diff = s.into();
    let tokens_length = count_token(&*diff);
    match tokens_length.cmp(&(4096 as usize)) {
        Ordering::Greater => {
            println!("{} {}", "The diff is too long!".red(), format!("The diff is ~{} tokens long, while the maximum is 4096.", tokens_length).bright_black());
            let list_str = match get_staged_files() {
                Ok(list) => {
                    list
                }
                Err(e) => {
                    panic!("{}", e);
                }
            };
            let list = list_str.split("\n").filter(|s| !s.is_empty()).collect::<Vec<&str>>();
            let ans = MultiSelect::new("Select the files you want to include the diff from:", list)
                .prompt();
            let new_diff = match ans {
                Ok(ans) => {
                    match git_diff_from_files(ans) {
                        Ok(diff) => {
                            check_diff(diff)
                        }
                        Err(e) => {
                            panic!("{}", e);
                        }
                    }
                }
                Err(e) => {
                    panic!("{}", e);
                }
            };
            new_diff
        }
        _ => diff
    }
}

fn count_token(s: &str) -> usize {
    let bpe = cl100k_base().unwrap();
    let mut text = SYSTEM_MSG.to_string();
    text += "\n";
    text += s;
    let tokens = bpe.encode_with_special_tokens(&*text);
    tokens.len()
}

fn get_staged_files() -> Result<String, String> {
    let diff = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--name-only")
        .output()
        .expect("failed to execute git diff");
    match diff.status.success() {
        true => Ok(String::from_utf8_lossy(&diff.stdout).to_string().replace("\r\n", "\n")),
        false => Err(String::from_utf8_lossy(&diff.stderr).to_string()),
    }
}

fn git_diff() -> Result<String, String> {
    let diff = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--minimal")
        .arg("-U2")
        .output()
        .expect("failed to execute git diff");
    match diff.status.success() {
        true => Ok(String::from_utf8_lossy(&diff.stdout).to_string().replace("\r\n", "\n")),
        false => Err(String::from_utf8_lossy(&diff.stderr).to_string()),
    }
}

fn git_diff_from_files(v: Vec<&str>) -> Result<String, String> {
    let mut binding = Command::new("git");
    let cmd = binding
        .arg("diff")
        .arg("--staged")
        .arg("--minimal")
        .arg("-U2")
        .arg("--");
    for file in v {
        cmd.arg(file);
    }
    let diff = cmd
        .output()
        .expect("failed to execute git diff");
    match diff.status.success() {
        true => Ok(String::from_utf8_lossy(&diff.stdout).to_string().replace("\r\n", "\n")),
        false => Err(String::from_utf8_lossy(&diff.stderr).to_string()),
    }
}

const PRICE: f64 = 0.002;

fn cost(token: i64) -> f64 {
    token as f64 * (PRICE / 1000.0)
}

#[cfg(test)]
mod tests {
    use crate::count_token;

    #[test]
    fn test_simple_count_token() {
        let result = count_token("tiktoken is great!");
        assert_eq!(result, 83);
    }
}