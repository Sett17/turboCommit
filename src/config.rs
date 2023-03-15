use crate::cli;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_temperature: f64,
    pub default_frequency_penalty: f64,
    pub default_number_of_choices: i32,
    pub default_system_msg: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_temperature: 1.0,
            default_frequency_penalty: 0.0,
            default_number_of_choices: 1,
            default_system_msg: String::from("As an AI that only returns conventional commits, you will receive input from the user in the form of a git diff of all staged files. The user may provide extra information to explain the change. Focus on the why rather than the what and keep it brief. You CANNOT generate anything that is not a conventional commit and a commit message only has 1 head line and at most 1 body.
Ensure that all commits follow these guidelines

- Commits must start with a type, which is a noun like feat, fix, chore, et., followed by an optional scope, an optional ! for breaking changes, and a required terminal colon and space
- Use feat for new features and fix for bug fixes
- You may provide a scope after a type. The scope should be a noun describing a section of the codebase, surrounded by parentheses
- After the type/scope prefix, include a short description of the code changes. This description should be followed immediately by a colon and a space
- You may provide a longer commit body after the short description. Body should start one blank line after the description and can consist of any number of newline-separated paragraphs

Example
feat: add a new feature

This body describes the feature in more detail"),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = home::home_dir().map_or_else(
            || {
                println!("{}", "Unable to find home directory.".red());
                process::exit(1);
            },
            |path| path.join(".turbocommit.yaml"),
        );
        match std::fs::read_to_string(path) {
            Ok(config) => match serde_yaml::from_str(&config) {
                Ok(config) => config,
                Err(err) => {
                    println!(
                        "{}\n{}",
                        format!("Unable to parse config file: {}", err).red(),
                        "Using default config.".bright_black()
                    );
                    Default::default()
                }
            },
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        println!("{}", "Using default config.".bright_black());
                    }
                    _ => {
                        println!(
                            "{}\n{}",
                            format!("Unable to read config file: {}\n", err).red(),
                            "Using default config.".bright_black()
                        );
                    }
                }
                Default::default()
            }
        }
    }
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = home::home_dir().map_or_else(
            || {
                println!("{}", "Unable to find home directory.".red());
                process::exit(1);
            },
            |path| path.join(".turbocommit.yaml"),
        );
        let config = match serde_yaml::to_string(self) {
            Ok(config) => config,
            Err(err) => {
                println!("{}", format!("Unable to serialize config: {}", err).red());
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to serialize config",
                ));
            }
        };
        std::fs::write(path, config)
    }
    pub fn overwrite(&mut self, opts: &cli::Options) {
        if let Some(n) = opts.n {
            self.default_number_of_choices = n;
        }
        if let Some(t) = opts.t {
            self.default_temperature = t;
        }
        if let Some(f) = opts.f {
            self.default_frequency_penalty = f;
        }
    }
}
