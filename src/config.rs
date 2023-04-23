use crate::openai;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process;

#[derive(Debug, Serialize, Deserialize)]
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
            model: openai::Model::Gpt35Turbo,
            default_temperature: 0.8,
            default_frequency_penalty: 0.0,
            default_number_of_choices: 2,
            disable_print_as_stream: false,
            system_msg: String::from("AI: gen conv commit-msg. Input: git diff staged files, context, user instr. Task: focus purpose, brief, clear. Output: 1 msg (1 headline, ≤1 body) ONLY.
Commit-msg guidelines:
1. Start: type (feat, fix, refactor, chore, etc.), opt. scope, opt. ! (breaking), req. colon+space.
2. feat=new features, fix=bug fixes, etc.
3. Scope: codebase section, in ().
4. After type/scope: concise desc, colon+space.
5. Longer body: blank line after desc.

Multi-changes: 1 msg, concise. 📝Include all crucial changes. ⚠️ ONLY headline & body in output. No extra notes/comments/content.

```example
feat: add new feature

feature detail
```"),
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
