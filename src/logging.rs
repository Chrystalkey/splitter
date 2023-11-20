use serde::{Deserialize, Serialize};
use crate::split_logic::TransactionChange;

#[derive(Serialize, Deserialize)]
pub enum LoggedCommand {
    Split {
        name: String,
        amount: i64,
        from: Vec<String>,
        to: Vec<String>,
        group: String,
        balance_rest: bool,
    },
    Pay {
        amount: i64,
        group: String,
        from: String,
        to: String,
    },
    Undo(Box<LoggedCommand>),
    Create {
        name: String,
        members: Vec<String>,
    },
    DeleteGroup {
        group: String,
    },
    Balance {
        group: String
    },
}

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    command: LoggedCommand,
    change: TransactionChange,
}

impl LogEntry {
    pub fn new(cmd: LoggedCommand, chg: TransactionChange) -> Self {
        LogEntry {
            command: cmd,
            change: chg,
        }
    }

    /// the change vector "undo" action original vector + reversed = 0
    pub fn reversed_change(&self) -> TransactionChange {
        self.change.iter().map(|(k, &v)| (k.clone(), -v)).collect()
    }
}