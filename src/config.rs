use clap::{command, Parser, Subcommand};


#[derive(Parser, Debug)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<SubCommand>,

    #[arg(long, short)]
    pub(crate) database: Option<String>,
}

impl Cli {
    pub(crate) fn is_empty(&self) -> bool {
        self.command.is_none() && self.database.is_none()
    }
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
        group: Option<String>,
        index: Option<usize>,
    },
    Create {
        name: String,

        #[arg(short = 'a', long = "add")]
        members: Vec<String>,
    },
    DeleteGroup {
        group: String,
        #[arg(short = 'y', long = "yes")]
        yes: Option<bool>,
    },
    List {
        group: Option<String>,
        #[arg(short = 'a', long = "all")]
        all: Option<bool>,
    },
    Stat {
        group: Option<String>,
        all: Option<bool>,
    },
    Balance {
        group: String
    },
}