use clap::{Parser, Subcommand};


#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Option<SubCommand>,

    #[arg(long, short)]
    database: Option<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum SubCommand {
    Split {
        amount: f32,

        #[arg(long, short)]
        from: Vec<String>,

        #[arg(long, short)]
        to: Vec<String>,

        #[arg(long, short)]
        name: String,

        #[arg(long, short)]
        group: Option<String>,

        #[arg(long, short)]
        balance_rest: Option<bool>,
    },
    Pay {
        amount: f32,

        #[arg(long, short)]
        group: Option<String>,

        #[arg(long, short)]
        from: String,
        #[arg(long, short)]
        to: String,
    },
    Undo {
        index: Option<usize>
    },
    Create {
        name: String,

        #[arg(short = 'a', long = "add")]
        members: Vec<String>,
    },
    DeleteGroup {
        group: String
    },
    DeleteEntry {
        group: String,
        entry_number: usize,
    },
    List {
        group: Option<String>
    },
    Stat {
        group: Option<String>
    },
    Balance {
        group: String
    },
}