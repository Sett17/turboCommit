use colored::Colorize;
use config::Config;

use openai::Message;

use std::{env, process};

mod animation;
mod cli;
mod config;
mod git;
mod openai;
mod util;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load();
    match config.save() {
        Ok(_) => (),
        Err(err) => {
            println!("{}", format!("Unable to write to config: {err}").red());
            process::exit(1);
        }
    }
    let options = cli::Options::new(env::args(), &config);

    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
        process::exit(1);
    };

    let repo = git::get_repo()?;

    let system_len = openai::count_token(&config.system_msg).unwrap_or(0);
    let extra_len = openai::count_token(&options.msg).unwrap_or(0);

    let (diff, diff_tokens) =
        util::decide_diff(&repo, system_len + extra_len, options.model.context_size())?;

    if options.dry_run {
        println!("This will use ~{} prompt tokens, costing you ~${}.\nEach 1K completion tokens will cost you ~${}",
            format!("{}", system_len + extra_len + diff_tokens).purple(),
            format!("{:0.5}", options.model.cost(system_len + extra_len + diff_tokens, 0)).purple(),
            format!("{:0.5}", options.model.cost(0, 1000)).purple());
        util::check_version().await;
        process::exit(0);
    }

    let prompt_tokens = system_len + extra_len + diff_tokens;

    let mut messages = vec![Message::system(config.system_msg), Message::user(diff)];

    if !options.msg.is_empty() {
        messages.push(Message::user(options.msg));
    }

    let choices = openai::Request::new(
        options.model.clone().to_string(),
        messages,
        options.n,
        options.t,
        options.f,
    )
    .execute(
        api_key,
        options.print_once,
        options.model.clone(),
        prompt_tokens,
    )
    .await?;

    let mut chosen_message = util::choose_message(choices);

    util::user_action(chosen_message)?;

    util::check_version().await;

    Ok(())
}
