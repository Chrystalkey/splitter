use clap::Parser;
use crate::config::Cli;

mod db_connect;
mod split_logic;
mod config;
mod test_db_connect;


fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
}
