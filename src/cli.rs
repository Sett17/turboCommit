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
                "-m" => {
                    if let Some(m) = iter.next() {
                        opts.msg = m;
                    }
                }
                "-h" => help(),
                "--help" => help(),
                _ => {}
            }
        }
        opts
    }
}

fn help() {
    println!(
        "
   __             __                   
  / /___  _______/ /_  ____            
 / __/ / / / ___/ __ \\/ __ \\           
/ /_/ /_/ / /  / /_/ / /_/ /           
\\__/\\__,_/_/  /_.___/\\____/       _ __ 
  _________  ____ ___  ____ ___  (_) /_
 / ___/ __ \\/ __ `__ \\/ __ `__ \\/ / __/
/ /__/ /_/ / / / / / / / / / / / / /_  
\\___/\\____/_/ /_/ /_/_/ /_/ /_/_/\\__/  
                                       
"
    );
    println!("Usage: turbocommit [options]");
    println!("Options:");
    println!("  -n <n>   Number of choices to generate (default: 1)");
    println!("  -m <msg> Extra message passed to the AI");
    process::exit(1);
}
