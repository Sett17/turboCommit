use std::cmp::Ordering;
use std::error::Error;
use std::process::Command;
use std::{env, process};

use colored::Colorize;
use inquire::MultiSelect;
use tiktoken_rs::tiktoken::cl100k_base;

use openai::Message;

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
            process::exit(1);
        }
    };

    if !is_git_repo() {
        println!(
            "{} {}",
            "Not a git repository.".red(),
            "Please run this command in a git repository.".bright_black()
        );
        process::exit(1);
    }

    println!();
    let full_diff = match git_diff() {
        Ok(diff) => diff,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    // check if diff is empty
    if full_diff.trim().is_empty() {
        println!(
            "{} {}",
            "No staged files.".red(),
            "Please stage the files you want to commit.".bright_black()
        );
        process::exit(1);
    }

    let diff = match check_diff(full_diff) {
        Ok(diff) => diff,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    let messages = vec![Message::system(SYSTEM_MSG), Message::user(diff)];

    let req = openai::Request::new(MODEL, messages);

    let json = match serde_json::to_string(&req) {
        Ok(json) => json,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .body(json)
        .send();

    match res {
        Ok(res) => match res.status() {
            reqwest::StatusCode::OK => {
                let body = match res.text() {
                    Ok(body) => body,
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                };
                let resp = match serde_json::from_str::<openai::Response>(&body) {
                    Ok(resp) => resp,
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                };
                println!(
                    "This used {} token, costing you ~{}$",
                    format!("{}", resp.usage.total_tokens).green(),
                    format!("{}", cost(resp.usage.total_tokens)).green()
                );
                for choice in resp.choices {
                    println!("===============================");
                    println!("{}", choice.message.content);
                    println!("===============================");
                }
            }
            _ => {
                let e = match res.text() {
                    Ok(e) => e,
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                };
                let error = match serde_json::from_str::<openai::ErrorRoot>(&e) {
                    Ok(error) => error.error,
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                };
                println!("{}", error);
            }
        },
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    }
}

fn check_diff<S: Into<String>>(s: S) -> Result<String, Box<dyn Error>> {
    let diff = s.into();
    let tokens_length = count_token(&diff)?;
    match tokens_length.cmp(&4096_usize) {
        Ordering::Greater => {
            println!(
                "{} {}",
                "The diff is too long!".red(),
                format!(
                    "The diff is ~{} tokens long, while the maximum is 4096.",
                    tokens_length
                )
                .bright_black()
            );
            let list_str = match get_staged_files() {
                Ok(list) => list,
                Err(e) => {
                    panic!("{}", e);
                }
            };
            let list = list_str
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>();
            let ans = MultiSelect::new("Select the files you want to include the diff from:", list)
                .prompt();

            match ans {
                Ok(ans) => match git_diff_from_files(ans) {
                    Ok(diff) => check_diff(diff),
                    Err(e) => {
                        panic!("{}", e);
                    }
                },
                Err(e) => {
                    panic!("{}", e);
                }
            }
        }
        _ => Ok(diff),
    }
}

fn count_token(s: &str) -> Result<usize, Box<dyn Error>> {
    let bpe = cl100k_base()?;
    let mut text = SYSTEM_MSG.to_string();
    text += "\n";
    text += s;
    let tokens = bpe.encode_with_special_tokens(&text);
    Ok(tokens.len())
}

fn is_git_repo() -> bool {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .expect("Failed to execute git command");
    output.status.success()
}

fn get_staged_files() -> Result<String, Box<dyn Error>> {
    let diff = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--name-only")
        .output()?;
    match diff.status.success() {
        true => Ok(String::from_utf8_lossy(&diff.stdout)
            .to_string()
            .replace("\r\n", "\n")),
        false => Err(Box::try_from(String::from_utf8_lossy(&diff.stderr).to_string()).unwrap()),
    }
}

fn git_diff() -> Result<String, Box<dyn Error>> {
    let diff = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--minimal")
        .arg("-U2")
        .output()?;
    match diff.status.success() {
        true => Ok(String::from_utf8_lossy(&diff.stdout)
            .to_string()
            .replace("\r\n", "\n")),
        false => Err(Box::try_from(String::from_utf8_lossy(&diff.stderr).to_string()).unwrap()),
    }
}

fn git_diff_from_files(v: Vec<&str>) -> Result<String, Box<dyn Error>> {
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
    let diff = cmd.output()?;
    match diff.status.success() {
        true => Ok(String::from_utf8_lossy(&diff.stdout)
            .to_string()
            .replace("\r\n", "\n")),
        false => Err(Box::try_from(String::from_utf8_lossy(&diff.stderr).to_string()).unwrap()),
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
        let result = match count_token("tiktoken is great!") {
            Ok(result) => result,
            Err(e) => {
                panic!("{}", e);
            }
        };
        assert_eq!(result, 83);
    }
}
