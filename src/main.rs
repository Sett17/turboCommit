use std::{env, process};

use colored::Colorize;

use inquire::validator::Validation;
use inquire::{Confirm, CustomUserError};

use crate::git::commit;
use openai::Message;

mod cli;
mod git;
mod openai;

const SYSTEM_MSG: &str = "As an AI that only returns conventional commits, you will receive input from the user in the form of a git diff of all staged files. The user may provide extra information to explain the change. Focus on the why rather than the what and keep it brief. You CANNOT generate anything that is not a conventional commit and a commit message only has 1 head line and at most 1 body.
Ensure that all commits follow these guidelines

- Commits must start with a type, which is a noun like feat, fix, chore, et., followed by an optional scope, an optional ! for breaking changes, and a required terminal colon and space
- Use feat for new features and fix for bug fixes
- You may provide a scope after a type. The scope should be a noun describing a section of the codebase, surrounded by parentheses
- After the type/scope prefix, include a short description of the code changes. This description should be followed immediately by a colon and a space
- You may provide a longer commit body after the short description. Body should start one blank line after the description and can consist of any number of newline-separated paragraphs

Example
feat: add a new feature

This body describes the feature in more detail";
const MODEL: &str = "gpt-3.5-turbo";

fn main() {
    let options = cli::Options::new(env::args());

    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
        process::exit(1);
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
            println!("{e}");
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

    let system_len = openai::count_token(SYSTEM_MSG).unwrap_or(0);
    let extra_len = openai::count_token(&options.msg).unwrap_or(0);

    let diff = match git::check_diff(&full_diff, system_len, extra_len) {
        Ok(diff) => diff,
        Err(e) => {
            println!("{e}");
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
            println!("{e}");
            process::exit(1);
        }
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .body(json)
        .send();

    match response {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::OK {
                let body = match response.text() {
                    Ok(body) => body,
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                };
                let resp = match serde_json::from_str::<openai::Response>(&body) {
                    Ok(resp) => resp,
                    Err(e) => {
                        println!("error parsing response: {e}\n {body:?}");
                        process::exit(1);
                    }
                };
                println!(
                    "This used {} token, costing you ~{}$",
                    format!("{}", resp.usage.total_tokens).green(),
                    format!("{}", openai::cost(resp.usage.total_tokens)).green()
                );
                for (i, choice) in resp.choices.iter().enumerate() {
                    println!("\n[{i}]============================");
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
                            println!("{e}");
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
                                println!("{e}");
                                process::exit(1);
                            }
                        }
                    } else {
                        process::exit(0);
                    }
                }
                let max_index = resp.choices.len();
                let commit_index = match inquire::CustomType::<usize>::new(
                    "Which commit message do you want to use? ",
                )
                .with_validator(move |i: &usize| {
                    if *i >= max_index {
                        Err(CustomUserError::from("Invalid index"))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()
                {
                    Ok(i) => i,
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                };
                let commit_msg = resp.choices[commit_index].message.content.clone();
                match commit(commit_msg) {
                    Ok(_) => {
                        println!("\n{} 🎉", "Commit successful!".green());
                        process::exit(0);
                    }
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                }
            } else {
                let e = match response.text() {
                    Ok(e) => e,
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                };
                let error = match serde_json::from_str::<openai::ErrorRoot>(&e) {
                    Ok(error) => error.error,
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                };
                println!("{error}");
            }
        }
        Err(e) => {
            println!("{e}");
            process::exit(1);
        }
    }
}
