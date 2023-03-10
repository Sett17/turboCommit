use crate::openai::count_token;
use crate::SYSTEM_MSG;
use colored::Colorize;
use std::{cmp, env, process};

#[derive(Debug)]
pub struct Options {
    pub n: i32,
    pub msg: String,
    pub t: f64,
    pub f: f64,
}

impl Options {
    pub fn new(args: env::Args) -> Self {
        let mut opts = Self {
            n: 1,
            msg: String::new(),
            t: 1.0,
            f: 0.0,
        };
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
                            |t| {
                                if t < 0.0 {
                                    0.0
                                } else if t > 2.0 {
                                    2.0
                                } else {
                                    t
                                }
                            },
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
                            |f| {
                                if f < -2.0 {
                                    -2.0
                                } else if f > 2.0 {
                                    2.0
                                } else {
                                    f
                                }
                            },
                        );
                    }
                }
                "-h" | "--help" => help(),
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
    println!("  -n <n>   Number of choices to generate (default: 1)\n");
    println!(
        "  -t <t>   Temperature (|t| 0.0 < t < 2.0) (default: 1.0) {}\n",
        "(https://platform.openai.com/docs/api-reference/chat/create#chat/create-temperature)"
            .bright_black()
    );
    println!("Anything else will be concatenated into an extra message given to the AI");
    println!(
        "\nThe prompt is about ~{} tokens long",
        format!("{}", count_token(SYSTEM_MSG).unwrap_or(0)).green()
    );
    process::exit(1);
}
