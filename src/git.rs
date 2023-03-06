use crate::openai;
use colored::Colorize;
use inquire::MultiSelect;
use std::cmp::Ordering;
use std::error::Error;
use std::process::Command;

pub fn check_diff<S: Into<String>>(s: S, extra: &String) -> Result<String, Box<dyn Error>> {
    let diff = s.into();
    let tokens_length = openai::count_token(&diff, extra)?;
    match tokens_length.cmp(&4096_usize) {
        Ordering::Greater => {
            println!(
                "{} {}",
                "The request is too long!".red(),
                format!(
                    "The request is ~{} tokens long, while the maximum is 4096.",
                    tokens_length
                )
                .bright_black()
            );
            let list_str = match staged_files() {
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
                Ok(ans) => match diff_from_files(ans) {
                    Ok(diff) => check_diff(diff, extra),
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

pub fn is_repo() -> bool {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .expect("Failed to execute git command");
    output.status.success()
}

fn staged_files() -> Result<String, Box<dyn Error>> {
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

pub fn diff() -> Result<String, Box<dyn Error>> {
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

fn diff_from_files(v: Vec<&str>) -> Result<String, Box<dyn Error>> {
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
