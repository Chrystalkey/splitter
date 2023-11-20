use clap::Parser;
use crate::config::Cli;
use crate::split_logic::Logic;

mod split_logic;
mod config;
mod error;
mod money;
mod logging;

fn main() {
    let cli = Cli::parse();
    if cli.is_empty() {
        todo!("Here you should enter an interactive command mode, still under development");
    } else {
        let mut logic = Logic::new(cli.database.unwrap_or(String::new()).into());
        logic.run(cli.command.unwrap());
        logic.save().expect("Could not save the Internal State to disk");
    }
}
