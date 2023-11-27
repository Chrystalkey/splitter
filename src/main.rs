use std::fs;
use clap::Parser;
use crate::config::Cli;
use crate::logic::Splitter;

mod logic;
mod config;
mod error;
mod money;
mod logging;
mod group;

fn main() {
    let cli = Cli::parse();
    if cli.is_empty() {
        todo!("Here you should enter an interactive command mode, still under development");
    } else {
        let dbpath = if cli.database.is_none() {
            let home = dirs::home_dir().expect("Could not find a home directory. Please explicitly specify a database");
            let splitter_home = home.join(".config/splitter");
            if !splitter_home.exists() {
                fs::create_dir_all(splitter_home.clone()).expect("Could not create ~/.config/splitter");
            }
            splitter_home.join("default.db")
        } else {
            cli.database.unwrap().into()
        };
        let mut logic = Splitter::new(dbpath);
        logic.run(cli.command.unwrap());
        logic.save().expect("Could not save the Internal State to disk");
    }
}
