use crate::config::Config;
use crate::openai;
use crate::openai::count_token;
use colored::Colorize;
use std::str::FromStr;
use std::{cmp, env, process};

#[derive(Debug)]
pub struct Options {
    pub n: i32,
    pub msg: String,
    pub t: f64,
    pub f: f64,
    pub model: openai::Model,
    pub dry_run: bool,
}

impl From<&Config> for Options {
    fn from(config: &Config) -> Self {
        Self {
            n: config.default_number_of_choices,
            msg: String::new(),
            t: config.default_temperature,
            f: config.default_frequency_penalty,
            model: config.model,
            dry_run: false,
        }
    }
}

impl Options {
    pub fn new(args: env::Args, conf: &Config) -> Self {
        let mut opts = Self::from(conf);
        let mut iter = args.skip(1);
        let mut msg = String::new();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-n" => {
                    if let Some(n) = iter.next() {
                        opts.n = n.parse().map_or_else(
                            |_| {
                                println!(
                                    "{} {}",
                                    "Could not parse n.".red(),
                                    "Please enter an integer.".bright_black()
                                );
                                process::exit(1);
                            },
                            |n| cmp::max(1, n),
                        );
                    }
                }
                "-t" => {
                    if let Some(t) = iter.next() {
                        opts.t = t.parse().map_or_else(
                            |_| {
                                println!(
                                    "{} {}",
                                    "Could not parse t.".red(),
                                    "Please enter a float between 0 and 2.".bright_black()
                                );
                                process::exit(1);
                            },
                            |t: f64| t.clamp(0.0, 2.0),
                        );
                    }
                }
                "-f" => {
                    if let Some(f) = iter.next() {
                        opts.f = f.parse().map_or_else(
                            |_| {
                                println!(
                                    "{} {}",
                                    "Could not parse f.".red(),
                                    "Please enter a float between -2.0 and 2.0.".bright_black()
                                );
                                process::exit(1);
                            },
                            |f: f64| f.clamp(-2.0, 2.0),
                        );
                    }
                }
                "-m" | "--model" => {
                    if let Some(model) = iter.next() {
                        opts.model = match openai::Model::from_str(&model) {
                            Ok(model) => model,
                            Err(err) => {
                                println!(
                                    "{} {}",
                                    format!("Could not parse model: {}", err).red(),
                                    "Please enter a valid model.".bright_black()
                                );
                                process::exit(1);
                            }
                        };
                    }
                }
                "-d" | "--dry-run" => {
                    opts.dry_run = true;
                    println!(
                        "{}",
                        "Dry run. Will not ask AI for completions".bright_black()
                    );
                }
                "-h" | "--help" => help(),
                "-v" | "--version" => {
                    println!("turbocommit version {}", env!("CARGO_PKG_VERSION").purple());
                    process::exit(0);
                }
                _ => {
                    if arg.starts_with('-') {
                        println!(
                            "{} {} {}",
                            "Unknown option: ".red(),
                            arg.purple().bold(),
                            "\nPlease use -h or --help for help.".bright_black()
                        );
                        process::exit(1);
                    }
                    msg.push_str(&arg);
                    msg.push(' ');
                }
            }
        }
        opts.msg = msg.trim().to_string();
        opts
    }
}

fn help() {
    println!("{}", "    __             __".red());
    println!("{}", "   / /___  _______/ /_  ____".red());
    println!("{}", "  / __/ / / / ___/ __ \\/ __ \\".yellow());
    println!("{}", " / /_/ /_/ / /  / /_/ / /_/ /".green());
    println!(
        "{}{}",
        " \\__/\\__,_/_/  /_.___/\\____/       ".blue(),
        "_ __".purple()
    );
    println!("{}", "   _________  ____ ___  ____ ___  (_) /_".purple());
    println!("{}", "  / ___/ __ \\/ __ `__ \\/ __ `__ \\/ / __/".red());
    println!("{}", " / /__/ /_/ / / / / / / / / / / / / /_".yellow());
    println!("{}", " \\___/\\____/_/ /_/ /_/_/ /_/ /_/_/\\__/".green());

    println!("\nUsage: turbocommit [options] [message]\n");
    println!("Options:");
    println!("  -n <n>   Number of choices to generate\n",);
    println!("  -m <m>   Model to use\n  --model <m>\n",);
    println!("  -d       Dry run. Will not ask AI for completions\n  --dry-run\n",);
    println!(
        "  -t <t>   Temperature (|t| 0.0 < t < 2.0)\n{}\n",
        "(https://platform.openai.com/docs/api-reference/chat/create#chat/create-temperature)"
            .bright_black()
    );
    println!(
        "  -f <f>   Frequency penalty (|f| -2.0 < f < 2.0)\n{}\n",
        "(https://platform.openai.com/docs/api-reference/chat/create#chat/create-frequency-penalty)"
            .bright_black()
    );
    println!("Anything else will be concatenated into an extra message given to the AI\n");
    println!("You can change the default for these options and the system message prompt in the config file, that is created the first time running the program\n{}",
        home::home_dir().unwrap_or_else(|| "".into()).join(".turbocommit.yaml").display());
    println!("To go back to the default system message, delete the config file.\n");
    println!(
        "\nThe system message is about ~{} tokens long",
        format!(
            "{}",
            count_token(&crate::config::Config::default().system_msg).unwrap_or(0)
        )
        .green()
    );
    process::exit(1);
}
