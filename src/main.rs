use std::{env, process};

use colored::Colorize;

use inquire::validator::Validation;
use inquire::{Confirm, CustomUserError, MultiSelect};

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
            println!("{}", format!("Unable to write to config: {err}").red());
            process::exit(1);
        }
    }
    config.overwrite(&options);

    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
        process::exit(1);
    };

    let repo = match git::get_repo() {
        Ok(repo) => repo,
        Err(e) => {
            println!("{}", format!("{e}").red());
            process::exit(1);
        }
    };

    let staged_files = match git::staged_files(&repo) {
        Ok(staged_file) => staged_file,
        Err(e) => {
            println!("{}", format!("{e}").red());
            process::exit(1);
        }
    };

    let full_diff = match git::diff(&repo, &staged_files) {
        Ok(diff) => diff,
        Err(e) => {
            println!("{}", format!("{e}").red());
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

    let system_len = openai::count_token(&config.default_system_msg).unwrap_or(0);
    let extra_len = if options.msg.as_ref().is_some() && !options.msg.as_ref().unwrap().is_empty() {
        openai::count_token(options.msg.as_ref().unwrap()).unwrap_or(0)
    } else {
        0
    };

    let mut diff = full_diff;
    let mut diff_tokens = match openai::count_token(&diff) {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("{}", format!("{e}").red());
            process::exit(1);
        }
    };

    while system_len + extra_len + diff_tokens > 4096 {
        println!(
            "{} {}",
            "The request is too long!".red(),
            format!(
                "The request is ~{} tokens long, while the maximum is 4096.",
                system_len + extra_len + diff_tokens
            )
            .bright_black()
        );
        let selected_files = match MultiSelect::new(
            "Select the files you want to include in the diff:",
            staged_files.clone(),
        )
        .prompt()
        {
            Ok(selected_files) => selected_files,
            Err(e) => {
                println!("{}", format!("{e}").red());
                process::exit(1);
            }
        };
        diff = match git::diff(&repo, &selected_files) {
            Ok(diff) => diff,
            Err(e) => {
                println!("{}", format!("{e}").red());
                process::exit(1);
            }
        };
        diff_tokens = match openai::count_token(&diff) {
            Ok(tokens) => tokens,
            Err(e) => {
                println!("{}", format!("{e}").red());
                process::exit(1);
            }
        };
    }

    let mut messages = vec![
        Message::system(config.default_system_msg),
        Message::user(diff),
    ];

    if !options.msg.as_ref().unwrap_or(&String::new()).is_empty() {
        messages.push(Message::user(options.msg.unwrap_or_default()));
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

    println!("{}", "Asking AI...".bright_black());

    let start = std::time::Instant::now();
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
                let duration = start.elapsed();
                println!(
                    "{} {}",
                    "request took".bright_black(),
                    format!("{}.{:03}s", duration.as_secs(), duration.subsec_millis()).purple()
                );
                println!(
                    "This used {} token, costing you ~{}$",
                    format!("{}", resp.usage.total_tokens).purple(),
                    format!("{:0.5}", openai::cost(resp.usage.total_tokens)).purple()
                );
                for (i, choice) in resp.choices.iter().enumerate() {
                    println!(
                        "\n{}",
                        format!("[{}]============================", i.to_string().purple())
                            .bright_black()
                    );
                    println!("{}", choice.message.content);
                }
                println!("{}", "\n================================".bright_black());
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
                        match git::commit(resp.choices[0].message.content.clone()) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("{e}");
                                process::exit(1);
                            }
                        };
                        println!("{} 🎉", "Commit successful!".purple());
                    }
                    process::exit(0);
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
                match git::commit(commit_msg) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                };
                println!("{} 🎉", "Commit successful!".purple());
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
