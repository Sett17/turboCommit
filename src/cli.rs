use colored::Colorize;
use std::{cmp, env, process};

#[derive(Debug)]
pub struct Options {
    pub n: i32,
    pub msg: String,
}

impl Options {
    pub fn new(args: env::Args) -> Options {
        let mut opts = Options {
            n: 1,
            msg: String::new(),
        };
        let mut iter = args.skip(1);
        let mut msg = String::new();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-n" => {
                    if let Some(n) = iter.next() {
                        opts.n = match n.parse() {
                            Ok(n) => cmp::max(1, n),
                            Err(_) => {
                                println!(
                                    "{} {}",
                                    "Could not parse n.".red(),
                                    "Please enter an integer.".bright_black()
                                );
                                process::exit(1);
                            }
                        };
                    }
                }
                "-h" => help(),
                "--help" => help(),
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
    println!("Anything else will be concatenated into an extra message given to the AI");
    process::exit(1);
}
