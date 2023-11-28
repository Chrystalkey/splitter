use serde::{Deserialize, Serialize};
use crate::logic::{Money, Target, TransactionChange};
use crate::money::Currency;

#[derive(Serialize, Deserialize)]
pub(crate) enum LoggedCommand {
    Split {
        name: String,
        amount: Money,
        from: Vec<Target>,
        to: Vec<Target>,
        group: String,
        balance_rest: bool,
    },
    Pay {
        amount: Money,
        from: String,
        to: String,
    },
    Undo(Box<LoggedCommand>),
    Create {
        name: String,
        members: Vec<String>,
    },
}

impl LoggedCommand {
    fn to_string(&self, curr: Currency) -> String {
        match self {
            Self::Pay { from, to, amount } => {
                format!("pay: {}\t to {}\t: {}{}", from, to, *amount as f32 / curr.subdivision(), curr)
            }
            Self::Create { name, members } => {
                format!("create: group `{}` with members {:#?}", name, members)
            }
            Self::Split { name, amount, from, to, group, balance_rest } => {
                let from = from.iter()
                    .map(|t|
                        if t.amount.is_none() {
                            format!("{}: *", t.member)
                        } else {
                            format!("{}: {:.02}{}\n", t.member, t.amount.unwrap() as f32 / curr.subdivision(), curr)
                        }
                    )
                    .fold("".to_string(), |accu, el| format!("{}{}", accu, el));
                let to = to.iter()
                    .map(|t|
                        format!("{}: {:.02}{}\n", t.member, t.amount.unwrap() as f32 / curr.subdivision(), curr)
                    )
                    .fold("".to_string(), |accu, el| format!("{}{}", accu, el));
                if to.is_empty() {
                    format!("split: in group {} `{} {}{} payed for by\n{}\n{}",
                            group, name, *amount as f32 / curr.subdivision(), curr, from,
                            if *balance_rest { ", balancing the rest" } else { "" })
                } else {
                    format!("split: in group {} `{} {}{} payed for by\n{}\nto\n{}{}",
                            group, name, *amount as f32 / curr.subdivision(), curr, from, to,
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
    pub fn to_string(&self, curr: Currency) -> String {
        self.command.to_string(curr)
    }
}