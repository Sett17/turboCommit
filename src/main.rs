use std::{env, process};

use colored::Colorize;

use openai::Message;

mod cli;
mod git;
mod openai;

const SYSTEM_MSG: &str = "You are now an AI that writes conventional commits. The user will give you input in the form of a git diff of all the staged files, and may give you some extra information. Focus more on the why then the what. You shall only answer with the commit message in this format:
<type>[optional scope]: <description>

[optional body]
If the change is a breaking change, put a ! before the :";
const MODEL: &str = "gpt-3.5-turbo";

fn main() {
    let options = cli::Options::new(env::args());

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(api_key) => api_key,
        Err(_) => {
            println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
            process::exit(1);
        }
    };

    if !git::is_repo() {
        println!(
            "{} {}",
            "Not a git repository.".red(),
            "Please run this command in a git repository.".bright_black()
        );
        process::exit(1);
    }

    println!();
    let full_diff = match git::diff() {
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

    let diff = match git::check_diff(full_diff, &options.msg) {
        Ok(diff) => diff,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    let mut messages = vec![Message::system(SYSTEM_MSG), Message::user(diff)];

    if !options.msg.is_empty() {
        messages.push(Message::user(options.msg));
    }

    let req = openai::Request::new(MODEL, messages, options.n);

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
                // println!("{:?}", body);
                let resp = match serde_json::from_str::<openai::Response>(&body) {
                    Ok(resp) => resp,
                    Err(e) => {
                        println!("error parsing response: {}", e);
                        process::exit(1);
                    }
                };
                println!(
                    "This used {} token, costing you ~{}$",
                    format!("{}", resp.usage.total_tokens).green(),
                    format!("{}", openai::cost(resp.usage.total_tokens)).green()
                );
                for choice in resp.choices {
                    println!("===============================");
                    println!("{}", choice.message.content);
                }
                println!("===============================");
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
