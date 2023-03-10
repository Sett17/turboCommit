use crate::openai;

use colored::Colorize;
use inquire::MultiSelect;
use std::cmp::Ordering;
use std::process;
use std::process::{Command, Output};

pub fn check_diff(s: &str, system_len: usize, extra_len: usize) -> anyhow::Result<String> {
    let tokens_length = openai::count_token(s)?;
    match (tokens_length + system_len + extra_len).cmp(&4096_usize) {
        Ordering::Greater => {
            println!(
                "{} {}",
                "The request is too long!".red(),
                format!(
                    "The request is ~{} tokens long, while the maximum is 4096.",
                    tokens_length + system_len + extra_len
                )
                .bright_black()
            );
            let list_str = staged_files();
            let list = list_str
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>();
            let ans = MultiSelect::new("Select the files you want to include the diff from:", list)
                .prompt();

            match ans {
                Ok(ans) => check_diff(&diff_from_files(ans), system_len, extra_len),
                Err(e) => {
                    println!("{e}");
                    process::exit(1);
                }
            }
        }
        _ => Ok(s.parse()?),
    }
}

pub fn is_repo() -> bool {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .map_or_else(
            |e| {
                println!(
                    "{} {}",
                    "Error while running git:".red(),
                    format!("{}.", e).bright_black()
                );
                process::exit(1);
            },
            |o| o,
        );
    output.status.success()
}

fn staged_files() -> String {
    Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--name-only")
        .output()
        .map_or_else(
            |e| {
                println!(
                    "{} {}",
                    "Error while running git:".red(),
                    format!("{}.", e).bright_black()
                );
                process::exit(1);
            },
            |o| {
                String::from_utf8_lossy(&o.stdout)
                    .to_string()
                    .replace("\r\n", "\n")
            },
        )
}

pub fn diff() -> String {
    Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--minimal")
        .arg("-U2")
        .output()
        .map_or_else(
            |e| {
                println!(
                    "{} {}",
                    "Error while running git:".red(),
                    format!("{}.", e).bright_black()
                );
                process::exit(1);
            },
            |o| {
                String::from_utf8_lossy(&o.stdout)
                    .to_string()
                    .replace("\r\n", "\n")
            },
        )
}

fn diff_from_files(v: Vec<&str>) -> String {
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
    cmd.output().map_or_else(
        |e| {
            println!(
                "{} {}",
                "Error while running git:".red(),
                format!("{}.", e).bright_black()
            );
            process::exit(1);
        },
        |o| {
            String::from_utf8_lossy(&o.stdout)
                .to_string()
                .replace("\r\n", "\n")
        },
    )
}

pub fn commit(msg: String) {
    let output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(msg)
        .output()
        .map_or_else(
            |e| {
                println!(
                    "{} {}",
                    "Error while running git:".red(),
                    format!("{}.", e).bright_black()
                );
                process::exit(1);
            },
            |_| (),
        );
}
