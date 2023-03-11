use std::{env, process};

use colored::Colorize;

use inquire::validator::Validation;
use inquire::{Confirm, CustomUserError};

use config::Config;
use openai::Message;

mod cli;
mod config;
mod git;
mod openai;

const MODEL: &str = "gpt-3.5-turbo";

fn main() {
    let options = cli::Options::new(env::args());
    let mut config = Config::load();
    match config.save() {
        Ok(_) => (),
        Err(err) => {
            println!("{}", format!("Unable to write to config: {}", err).red());
            process::exit(1);
        }
    }
    config.overwrite(&options);

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
    let full_diff = git::diff();

    if full_diff.trim().is_empty() {
        println!(
            "{} {}",
            "No staged files.".red(),
            "Please stage the files you want to commit.".bright_black()
        );
        process::exit(1);
    }

    let system_len = openai::count_token(&config.default_system_msg).unwrap_or(0);
    let extra_len =
        openai::count_token(options.msg.as_ref().unwrap_or(&String::from(""))).unwrap_or(0);

    let diff = match git::check_diff(&full_diff, system_len, extra_len) {
        Ok(diff) => diff,
        Err(e) => {
            println!("{e}");
            process::exit(1);
        }
    };

    let mut messages = vec![
        Message::system(config.default_system_msg),
        Message::user(diff),
    ];

    if !options.msg.as_ref().unwrap_or(&String::from("")).is_empty() {
        messages.push(Message::user(options.msg.unwrap_or(String::from(""))));
    }

    let req = openai::Request::new(
        String::from(MODEL),
        messages,
        options.n.unwrap_or(1),
        options.t.unwrap_or(1.0),
        options.f.unwrap_or(0.0),
    );

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
                    println!(
                        "\n{}",
                        format!("[{i}]============================").bright_black()
                    );
                    println!("{}", choice.message.content);
                }
                println!("{}", "================================".bright_black());
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
                        git::commit(resp.choices[0].message.content.clone());
                        println!("{} 🎉", "Commit successful!".green());
                        process::exit(0);
                    } else {
                        process::exit(0);
                    }
                }
                let max_index = resp.choices.len();
                let commit_index = match inquire::CustomType::<usize>::new(&format!(
                    "Which commit message do you want to use? {}",
                    "<ESC> to cancel".bright_black()
                ))
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
                git::commit(commit_msg);
                println!("{} 🎉", "Commit successful!".green());
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
