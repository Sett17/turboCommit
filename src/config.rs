use crate::openai;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub model: openai::Model,
    #[serde(default)]
    pub default_temperature: f64,
    #[serde(default)]
    pub default_frequency_penalty: f64,
    #[serde(default)]
    pub default_number_of_choices: i32,
    #[serde(default)]
    pub disable_print_as_stream: bool,
    #[serde(default)]
    pub system_msg: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: openai::Model::Gpt35Turbo16k,
            default_temperature: 0.8,
            default_frequency_penalty: 0.0,
            default_number_of_choices: 2,
            disable_print_as_stream: false,
            system_msg: String::from("As an AI that only returns conventional commits, you will receive input from the user in the form of a git diff of all staged files. Focus on the why rather than the what and keep it brief. You CANNOT generate anything that is not a conventional commit and a commit message only has 1 head line and at most 1 body.
Do NOT start the body with things like 'This commit ...' or similar, just start the body description.
Make sure the body reads as a single message, NOT a list of bullets or multiple commits.
The user may give you more specific instructions or extra information. The user may ask for revisions.
Ensure that all commits follow these guidelines

- Commits must start with a type, which is a noun like feat, fix, chore, etc., followed by an optional scope, an optional ! for breaking changes, and a required terminal colon and space
- Use feat for new features and fix for bug fixes
- You may provide a scope after a type. The scope should be a noun describing a section of the codebase, surrounded by parentheses
- After the type/scope prefix, include a short description of the code changes. This description should be followed immediately by a colon and a space
- You may provide a longer commit body after the short description. Body should start one blank line after the description and can consist of any number of newline-separated paragraphs

Example
feat: add a new feature

More detailed description about why"),
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
}
