use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::logic::{Target, TransactionChange};

#[derive(Serialize, Deserialize)]
pub(crate) enum LoggedCommand {
    Split {
        name: String,
        amount: i64,
        from: Vec<Target>,
        to: Vec<Target>,
        group: String,
        balance_rest: bool,
    },
    Pay {
        amount: i64,
        from: String,
        to: String,
    },
    Undo(Box<LoggedCommand>),
    Create {
        name: String,
        members: Vec<String>,
    },
}

impl Display for LoggedCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pay { from, to, amount } => {
                write!(f, "pay: {}\t to {}\t: {}€", from, to, *amount as f32 / 100.)
            }
            Self::Create { name, members } => {
                write!(f, "create: group `{}` with members {:#?}", name, members)
            }
            Self::Split { name, amount, from, to, group, balance_rest } => {
                let from = from.iter()
                    .map(|t|
                        if t.amount.is_none() {
                            format!("{}: *", t.member)
                        } else {
                            format!("{}: {:.02}€\n", t.member, t.amount.unwrap() as f32 / 100.)
                        }
                    )
                    .fold("".to_string(), |accu, el| format!("{}{}", accu, el));
                let to = to.iter()
                    .map(|t|
                        format!("{}: {:.02}€\n", t.member, t.amount.unwrap() as f32 / 100.)
                    )
                    .fold("".to_string(), |accu, el| format!("{}{}", accu, el));
                if to.is_empty() {
                    write!(f, "split: in group {} `{} {}€ payed for by\n{}\n{}",
                           group, name, *amount as f32 / 100., from,
                           if *balance_rest { ", balancing the rest" } else { "" })
                } else {
                    write!(f, "split: in group {} `{} {}€ payed for by\n{}\nto\n{}{}",
                           group, name, *amount as f32 / 100., from, to,
                           if *balance_rest { ", balancing the rest" } else { "" })
                }
            }
            Self::Undo(_) => { todo!("Undo is not implemented right now") }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LogEntry {
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

impl Display for LogEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command)
    }
}