use std::{env, process};

use colored::Colorize;

use inquire::validator::Validation;
use inquire::{Confirm, CustomUserError};

use crate::git::commit;
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
                let resp = match serde_json::from_str::<openai::Response>(&body) {
                    Ok(resp) => resp,
                    Err(e) => {
                        println!("error parsing response: {}\n {:?}", e, body);
                        process::exit(1);
                    }
                };
                println!(
                    "This used {} token, costing you ~{}$",
                    format!("{}", resp.usage.total_tokens).green(),
                    format!("{}", openai::cost(resp.usage.total_tokens)).green()
                );
                for (i, choice) in resp.choices.iter().enumerate() {
                    println!("\n[{}]============================", i);
                    println!("{}", choice.message.content);
                }
                println!("===============================");
                if resp.choices.len() == 1 {
                    let answer = match Confirm::new("Do you want to commit with this message? ")
                        .with_default(true)
                        .prompt()
                    {
                        Ok(answer) => answer,
                        Err(e) => {
                            println!("{}", e);
                            process::exit(1);
                        }
                    };
                    if answer {
                        match commit(resp.choices[0].message.content.clone()) {
                            Ok(_) => {
                                println!("\n{} 🎉", "Commit successful!".green());
                                process::exit(0);
                            }
                            Err(e) => {
                                println!("{}", e);
                                process::exit(1);
                            }
                        }
                    } else {
                        process::exit(0);
                    }
                }
                let max_index = resp.choices.len() as i32;
                let commit_index = match inquire::CustomType::<i32>::new(
                    "Which commit message do you want to use? ",
                )
                .with_validator(move |i: &i32| {
                    if *i < 0 || *i >= max_index {
                        Err(CustomUserError::from("Invalid index"))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()
                {
                    Ok(i) => i,
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
                };
                let commit_msg = resp.choices[commit_index as usize].message.content.clone();
                match commit(commit_msg) {
                    Ok(_) => {
                        println!("\n{} 🎉", "Commit successful!".green());
                        process::exit(0);
                    }
                    Err(e) => {
                        println!("{}", e);
                        process::exit(1);
                    }
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
