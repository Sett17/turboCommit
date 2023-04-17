use colored::Colorize;
use config::Config;
use crossterm::{
    cursor::{MoveToColumn, MoveToPreviousLine},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use futures::stream::StreamExt;
use inquire::{validator::Validation, Confirm, CustomUserError, MultiSelect, Select};
use openai::Message;

use reqwest_eventsource::{Event, EventSource};
use std::time::Duration;
use std::{env, process};
use unicode_segmentation::UnicodeSegmentation;

mod cli;
mod config;
mod git;
mod openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    match config.save() {
        Ok(_) => (),
        Err(err) => {
            println!("{}", format!("Unable to write to config: {err}").red());
            process::exit(1);
        }
    }
    let options = cli::Options::new(env::args(), &config);

    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
        process::exit(1);
    };

    let no_git_anim = options.print_once;
    let loading_git_animation = tokio::spawn(async move {
        if no_git_anim {
            println!("{}", "Extracting Information...".bright_black());
            return;
        }
        let emoji_support =
            terminal_supports_emoji::supports_emoji(terminal_supports_emoji::Stream::Stdout);
        let frames = if emoji_support {
            vec![
                "🕛", "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚",
            ]
        } else {
            vec!["/", "-", "\\", "|"]
        };
        let mut current_frame = 0;
        let mut stdout = std::io::stdout();
        loop {
            current_frame = (current_frame + 1) % frames.len();
            match execute!(
                stdout,
                Clear(ClearType::CurrentLine),
                MoveToColumn(0),
                SetForegroundColor(Color::Yellow),
                Print("Extracting Information ".bright_black()),
                Print(frames[current_frame]),
                ResetColor
            ) {
                Ok(_) => (),
                Err(_) => {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    });

    let repo = git::get_repo()?;
    let staged_files = git::staged_files(&repo)?;
    let full_diff = git::diff(&repo, &staged_files)?;

    if full_diff.trim().is_empty() {
        loading_git_animation.abort();
        execute!(
            std::io::stdout(),
            Clear(ClearType::CurrentLine),
            MoveToColumn(0),
        )?;
        println!(
            "{} {}",
            "No staged files.".red(),
            "Please stage the files you want to commit.".bright_black()
        );
        check_version().await;
        process::exit(0);
    }

    let system_len = openai::count_token(&config.system_msg).unwrap_or(0);
    let extra_len = openai::count_token(&options.msg).unwrap_or(0);

    let mut diff = full_diff;
    let mut diff_tokens = match openai::count_token(&diff) {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("{}", format!("{e}").red());
            process::exit(1);
        }
    };

    loading_git_animation.abort();
    execute!(
        std::io::stdout(),
        Clear(ClearType::CurrentLine),
        MoveToColumn(0),
    )?;
    while system_len + extra_len + diff_tokens > options.model.context_size() {
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

    if options.dry_run {
        println!("This will use ~{} prompt tokens, costing you ~${}.\nEach 1K completion tokens will cost you ~${}",
            format!("{}", system_len + extra_len + diff_tokens).purple(),
            format!("{:0.5}", options.model.cost(system_len + extra_len + diff_tokens, 0)).purple(),
            format!("{:0.5}", options.model.cost(0, 1000)).purple());
        check_version().await;
        process::exit(0);
    }

    let prompt_tokens = system_len + extra_len + diff_tokens;

    let mut messages = vec![Message::system(config.system_msg), Message::user(diff)];

    if !options.msg.is_empty() {
        messages.push(Message::user(options.msg));
    }

    let req = openai::Request::new(
        options.model.clone().to_string(),
        messages,
        options.n,
        options.t,
        options.f,
    );

    let json = match serde_json::to_string(&req) {
        Ok(json) => json,
        Err(e) => {
            println!("{e}");
            process::exit(1);
        }
    };

    let request_builder = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(json);

    let no_ai_anim = options.print_once;
    let loading_ai_animation = tokio::spawn(async move {
        if no_ai_anim {
            println!("{}", "Asking AI...".bright_black());
            return;
        }
        let emoji_support =
            terminal_supports_emoji::supports_emoji(terminal_supports_emoji::Stream::Stdout);
        let frames = if emoji_support {
            vec![
                "🕛", "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚",
            ]
        } else {
            vec!["/", "-", "\\", "|"]
        };
        let mut current_frame = 0;
        let mut stdout = std::io::stdout();
        loop {
            current_frame = (current_frame + 1) % frames.len();
            match execute!(
                stdout,
                Clear(ClearType::CurrentLine),
                MoveToColumn(0),
                SetForegroundColor(Color::Yellow),
                Print("Asking AI ".bright_black()),
                Print(frames[current_frame]),
                ResetColor
            ) {
                Ok(_) => {}
                Err(_) => {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    });

    let mut choices = vec![String::from(""); options.n as usize];

    let term_width = terminal::size()?.0 as usize;

    let mut stdout = std::io::stdout();

    let mut es = EventSource::new(request_builder)?;
    let mut lines_to_move_up = 0;
    let mut response_tokens = 0;

    while let Some(event) = es.next().await {
        if options.print_once {
            match event {
                Ok(Event::Message(message)) => {
                    if message.data == "[DONE]" {
                        break;
                    }
                    let resp = serde_json::from_str::<openai::Response>(&message.data)
                        .map_or_else(|_| openai::Response::default(), |r| r);
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
                    let resp = serde_json::from_str::<openai::Response>(&message.data)
                        .map_or_else(|_| openai::Response::default(), |r| r);
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
                                        options.model.cost(prompt_tokens, response_tokens)
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

    if options.print_once {
        println!(
            "This used {} tokens costing you about {}\n",
            format!("{}", response_tokens + prompt_tokens).purple(),
            format!(
                "~${:0.4}",
                options.model.cost(prompt_tokens, response_tokens)
            )
            .purple()
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

    let mut chosen_message = String::new();

    if choices.len() == 1 {
        chosen_message = choices[0].clone();
    } else {
        let max_index = choices.len();
        let commit_index = inquire::CustomType::<usize>::new(&format!(
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
        .prompt()?;
        chosen_message = choices[commit_index].clone();
    }

    let tasks = vec!["Commit it", "Edit it & Commit", "Abort"];

    let task = Select::new("What to do with the message?", tasks).prompt()?;

    match task {
        "Commit it" => {
            match git::commit(chosen_message) {
                Ok(_) => {}
                Err(e) => {
                    println!("{e}");
                    process::exit(1);
                }
            };
            println!("{} 🎉", "Commit successful!".purple());
        }
        "Edit it & Commit" => {
            let edited = edit::edit(chosen_message)?;
            execute!(
                stdout,
                Print(format!(
                    "{}\n",
                    format!("[{}]=======", "Edited Message".purple()).bright_black()
                )),
                Print(&edited),
                Print(format!("{}\n", "=======================".bright_black())),
            )?;
            let do_commit = Confirm::new("Do you want to commit with this message? ")
                .with_default(true)
                .prompt()?;
            if do_commit {
                match git::commit(edited) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                };
                println!("{} 🎉", "Commit successful!".purple());
            }
        }
        _ => {
            process::exit(0);
        }
    }
    check_version().await;

    Ok(())
}

async fn check_version() {
    let client = match crates_io_api::AsyncClient::new(
        "turbocommit lateste version checker",
        Duration::from_millis(1000),
    ) {
        Ok(client) => client,
        Err(_) => {
            return;
        }
    };
    let turbo = match client.get_crate("turbocommit").await {
        Ok(turbo) => turbo,
        Err(_) => {
            return;
        }
    };
    let newest_version = turbo.versions[0].num.clone();
    let current_version = env!("CARGO_PKG_VERSION");

    if current_version != newest_version {
        println!(
            "\n{} {}",
            "New version available!".yellow(),
            format!("v{}", newest_version).purple()
        );
        println!(
            "To update, run\n{}",
            "cargo install --force turbocommit".purple()
        );
    }
}

#[must_use]
pub fn count_lines(text: &str, max_width: usize) -> u16 {
    if text.is_empty() {
        return 0;
    }
    let mut line_count = 0;
    let mut current_line_width = 0;
    for cluster in UnicodeSegmentation::graphemes(text, true) {
        match cluster {
            "\r" | "\u{FEFF}" => {}
            "\n" => {
                line_count += 1;
                current_line_width = 0;
            }
            _ => {
                current_line_width += 1;
                if current_line_width > max_width {
                    line_count += 1;
                    current_line_width = cluster.chars().count();
                }
            }
        }
    }

    line_count + 1
}
