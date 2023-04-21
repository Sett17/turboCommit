use std::{process, time::Duration};

use colored::Colorize;
use crossterm::{execute, style::Print};
use inquire::{Confirm, MultiSelect, Select};
use unicode_segmentation::UnicodeSegmentation;

use crate::{git, openai};

pub fn decide_diff(
    repo: &git2::Repository,
    used_tokens: usize,
    context: usize,
) -> anyhow::Result<(String, usize)> {
    let staged_files = git::staged_files(&repo)?;
    let mut diff = git::diff(&repo, &staged_files)?;
    let mut diff_tokens = openai::count_token(&diff)?;

    if diff_tokens == 0 {
        println!(
            "{} {}",
            "No staged files.".red(),
            "Please stage the files you want to commit.".bright_black()
        );
        std::process::exit(1);
    }

    while used_tokens + diff_tokens > context {
        println!(
            "{} {}",
            "The request is too long!".red(),
            format!(
                "The request is ~{} tokens long, while the maximum is {}.",
                used_tokens + diff_tokens,
                context
            )
            .bright_black()
        );
        let selected_files = MultiSelect::new(
            "Select the files you want to include in the diff:",
            staged_files.clone(),
        )
        .prompt()?;
        diff = git::diff(&repo, &selected_files)?;
        diff_tokens = openai::count_token(&diff)?;
    }
    Ok((diff, diff_tokens))
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

pub async fn check_version() {
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

pub fn choose_message(choices: Vec<String>) -> String {
    if choices.len() == 1 {
        return choices[0].clone();
    }
    let max_index = choices.len();
    let commit_index = match inquire::CustomType::<usize>::new(&format!(
        "Which commit message do you want to use? {}",
        "<ESC> to cancel".bright_black()
    ))
    .with_validator(move |i: &usize| {
        if *i >= max_index {
            Err(inquire::CustomUserError::from("Invalid index"))
        } else {
            Ok(inquire::validator::Validation::Valid)
        }
    })
    .prompt() {
        Ok(index) => index,
        Err(_) => {
            process::exit(0);
        }
    };
    choices[commit_index].clone()
}

pub fn user_action(msg: String) -> anyhow::Result<()> {
    let tasks = vec!["Commit it", "Edit it & Commit", "Abort"];

    let task = Select::new("What to do with the message?", tasks).prompt()?;

    match task {
        "Commit it" => {
            match git::commit(msg) {
                Ok(_) => {}
                Err(e) => {
                    println!("{e}");
                    process::exit(1);
                }
            };
            println!("{} 🎉", "Commit successful!".purple());
        }
        "Edit it & Commit" => {
            let edited = edit::edit(msg)?;
            execute!(
                std::io::stdout(),
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
        _ => {}
    };
    Ok(())
}
