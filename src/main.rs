use clap::Parser;
use crate::config::Cli;

mod split_logic;
mod config;
mod error;
mod money;
mod logging;

fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
}
