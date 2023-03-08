use crate::openai;
use anyhow::anyhow;
use colored::Colorize;
use inquire::MultiSelect;
use std::cmp::Ordering;
use std::process;
use std::process::Command;

pub fn check_diff(s: &str, system_len: usize, extra_len: usize) -> anyhow::Result<String> {
    let tokens_length = openai::count_token(s)?;
    match (tokens_length + system_len + extra_len).cmp(&4096_usize) {
        Ordering::Greater => {
            println!(
                "{} {}",
                "The request is too long!".red(),
                format!("The request is ~{tokens_length} tokens long, while the maximum is 4096.")
                    .bright_black()
            );
            let list_str = match staged_files() {
                Ok(list) => list,
                Err(e) => {
                    println!("{e}");
                    process::exit(1);
                }
            };
            let list = list_str
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>();
            let ans = MultiSelect::new("Select the files you want to include the diff from:", list)
                .prompt();

            match ans {
                Ok(ans) => match diff_from_files(ans) {
                    Ok(diff) => check_diff(&diff, system_len, extra_len),
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                },
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
        .expect("Failed to execute git command");
    output.status.success()
}

fn staged_files() -> anyhow::Result<String> {
    let diff = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--name-only")
        .output()?;
    if diff.status.success() {
        Ok(String::from_utf8_lossy(&diff.stdout)
            .to_string()
            .replace("\r\n", "\n"))
    } else {
        Err(anyhow!(String::from_utf8_lossy(&diff.stderr).to_string()))
    }
}

pub fn diff() -> anyhow::Result<String> {
    let diff = Command::new("git")
        .arg("diff")
        .arg("--staged")
        .arg("--minimal")
        .arg("-U2")
        .output()?;
    if diff.status.success() {
        Ok(String::from_utf8_lossy(&diff.stdout)
            .to_string()
            .replace("\r\n", "\n"))
    } else {
        Err(anyhow!(String::from_utf8_lossy(&diff.stderr).to_string()))
    }
}

fn diff_from_files(v: Vec<&str>) -> anyhow::Result<String> {
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
    if diff.status.success() {
        Ok(String::from_utf8_lossy(&diff.stdout)
            .to_string()
            .replace("\r\n", "\n"))
    } else {
        Err(anyhow!(String::from_utf8_lossy(&diff.stderr).to_string()))
    }
}

pub fn commit(msg: String) -> anyhow::Result<()> {
    let output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(msg)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}
