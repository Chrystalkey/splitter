use std::collections::HashMap;
use std::io;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::string::ToString;
use std::thread::sleep;
use std::time::Duration;
use brotli::{CompressorReader, Decompressor};
use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::config::SubCommand;
use crate::error::*;
use crate::group::Group;

pub(crate) type Money = i64;

pub(crate) type TransactionChange = HashMap<String, Money>;


#[derive(Serialize, Deserialize)]
struct SplitterState {
    version: String,
    groups: Vec<Group>,
    current_group: Option<usize>,
}

impl SplitterState {
    fn new(source: PathBuf) -> Self {
        return if source.exists() {
            if source.is_file() {
                let raw = std::fs::read(source).unwrap();
                serde_yaml::from_str(
                    Splitter::decompress(raw.as_slice()).as_str()
                ).unwrap()
            } else {
                panic!("You specified '{:?}', which is not a file", source);
            }
        } else {
            if source == PathBuf::new() {
                println!("The Path to a persistent file is empty. \
                If you meant to only temporary store the result of this call, ignore the message");
            }
            Self {
                version: Splitter::CURRENT_VERSION.to_string(),
                groups: vec![],
                current_group: None,
            }
        };
    }

    /// get a reference to the group or panic
    fn get_group(&self, group_name: Option<String>) -> Result<&Group> {
        let group = match &group_name {
            None => self.groups.get(self.current_group.unwrap_or(0)),
            Some(name) => self.groups.iter().find(|&gn| gn.name == *name)
        };
        if let Some(g) = group {
            Ok(g)
        } else {
            Err(anyhow!(SplitterError::GroupNotFound))
                .context(group_name.unwrap_or("None".to_string()))
        }
    }
    fn get_group_mut(&mut self, group_name: Option<String>) -> Result<&mut Group> {
        let group = match &group_name {
            None => self.groups.get_mut(self.current_group.unwrap_or(0)),
            Some(name) => self.groups.iter_mut().find(|gn| gn.name == *name)
        };
        if let Some(g) = group {
            Ok(g)
        } else {
            Err(anyhow!(SplitterError::GroupNotFound)).context(group_name.unwrap_or("None".to_string()))
        }
    }
    fn get_group_idx(&mut self, group_name: Option<String>) -> Result<usize> {
        if self.groups.is_empty() {
            return Err(anyhow!(SplitterError::GroupNotFound))
                .context(group_name.unwrap_or("None".to_string()));
        }
        let gidx = match &group_name {
            None => self.current_group.unwrap_or(0),
            Some(name) => {
                let idx = self.groups.iter().enumerate().find(|(_, g)|
                    g.name == *name).map(|(n, _)| n);
                if idx.is_none() {
                    return Err(anyhow!(SplitterError::GroupNotFound))
                        .context(group_name.unwrap_or("None".to_string()));
                }
                idx.unwrap()
            }
        };
        Ok(gidx)
    }
    fn delete_group(&mut self, group_name: String, yes: bool) -> Result<()> {
        println!("This will delete the group '{}' forever with no more undo options available.\n",
                 group_name);
        {
            let group = self.get_group_mut(Some(group_name.clone()))?;
            group.stat();
        }
        let really = yes || Splitter::confirm();
        if really && !yes { // manually confirmed
            println!("Confirmed. Deleting group");
            self.groups = self.groups.drain(..)
                .filter(|grp| grp.name != group_name).collect();
            self.current_group = Some(0);
        } else if yes { // silent mode if yes was specified
            self.groups = self.groups.drain(..)
                .filter(|grp| grp.name != group_name).collect();
            self.current_group = Some(0);
        } else { // !confirm && !yes
            println!("Operation Cancelled");
        }
        self.current_group = None;
        Ok(())
    }
}

#[cfg(test)]
mod splitterstate_tests {
    use crate::group::Group;
    use super::*;

    #[test]
    fn test_delete_group_success() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(), "Charly".to_string()], None);
        let mut splitterstate = SplitterState {
            version: Splitter::CURRENT_VERSION.to_string(),
            groups: vec![group.unwrap()],
            current_group: Some(0),
        };
        assert_eq!(splitterstate.groups.len(), 1);
        let r = splitterstate.delete_group("testgroup".to_string(), true);
        assert!(r.is_ok());
        assert_eq!(splitterstate.groups.len(), 0);
    }

    #[test]
    fn test_delete_group_failure() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(), "Charly".to_string()], None);
        let mut splitterstate = SplitterState {
            version: Splitter::CURRENT_VERSION.to_string(),
            groups: vec![group.unwrap()],
            current_group: Some(0),
        };
        assert_eq!(splitterstate.groups.len(), 1);
        let r = splitterstate.delete_group("txt".to_string(), true);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err()
                       .downcast_ref(), Some(SplitterError::GroupNotFound).as_ref());
    }
}

/// helper struct containing money and a name. Can be used as a "from" or as a "to"
/// Can be parsed from --from/to {name}[:amount[%]]
#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub(crate) struct Target {
    pub(crate) member: String,
    pub(crate) amount: Option<i64>,
}

impl Target {
    /// Parses a target directive specified via `--from` or `--to` into a Target Struct
    fn parse(input: &str, total_money: i64) -> Result<Self> {
        let in_split: Vec<_> = input.trim_end_matches('%').split(':').collect();
        if in_split[0].is_empty() {
            return Err(anyhow!(SplitterError::InvalidTargetFormat))
                .context("Please use the format <name>:[<number>[%]]. (maybe you forgot ':'?");
        }
        if in_split.len() == 2 {
            let amount = if input.ends_with('%') {
                let percent: f32 = in_split[1].trim().replace(',', ".").parse::<f32>()?
                    / 100.;
                (percent * total_money as f32) as i64
            } else {
                let amount: f32 = in_split[1].trim().replace(',', ".").parse::<f32>()?
                    * 100.;
                amount.round() as i64
            };
            Ok(Self {
                member: in_split[0].to_owned(),
                amount: Some(amount),
            })
        } else if in_split.len() == 1 {
            if !Regex::new(Splitter::NAME_REGEX).unwrap().is_match(in_split[0]) {
                return Err(anyhow!(SplitterError::InvalidName))
                    .context(in_split[0].to_string())
                ;
            }
            Ok(Self {
                member: in_split[0].to_owned(),
                amount: None,
            })
        } else {
            Err(anyhow!(SplitterError::InvalidTargetFormat))
                .context("Please use the format <name>:[<number>[%]]. (maybe you forgot ':'?")
        }
    }
    /// Parses entries that originate with --from or --to arguments.
    /// it returns the list of the names together with an option denoting their amounts.
    /// None means they did not specify an amount.
    /// The second return value is the total amount that was explicitly given
    /// The third return value is the number of wildcard givers
    pub(crate) fn parse_multiple(raw_targets: Vec<String>, total_amount: i64) -> Result<(Vec<Target>, i64, usize)> {
        let mut targets_parsed = Vec::with_capacity(raw_targets.len());
        let mut summed = 0i64;
        let mut wildcard_givers = 0usize;
        for giver in &raw_targets {
            targets_parsed.push(Target::parse(giver.as_str(), total_amount)?);
            let t_amount = targets_parsed.last().unwrap().amount;
            summed += t_amount.unwrap_or(0);
            wildcard_givers += if t_amount.is_none() { 1 } else { 0 };
        }
        if summed.abs() > total_amount {
            return Err(anyhow!(SplitterError::LogicError))
                .context(format!("Error: The amounts specified with '--from' or '--to' sum up to more than the total amount: {} vs {}",
                                 summed, total_amount));
        }
        Ok((targets_parsed, summed, wildcard_givers))
    }
}

#[cfg(test)]
mod target_tests {
    use crate::logic::{Target};

    #[test]
    fn test_target_parse() {

        // valid cases
        let case_absolute_amount_comma = "peter:25,22";
        let ft = Target::parse(case_absolute_amount_comma, 100_00);
        assert!(ft.is_ok());
        let ft = ft.unwrap();
        assert_eq!(ft.member, "peter");
        assert_eq!(ft.amount.unwrap(), 25_22);

        let case_absolute_amount_dot = "peter:25.22";
        let ft = Target::parse(case_absolute_amount_dot, 100_00);
        assert!(ft.is_ok());
        let ft = ft.unwrap();
        assert_eq!(ft.member, "peter");
        assert_eq!(ft.amount.unwrap(), 25_22);


        let case_percentage = "peter:10%";
        let ft = Target::parse(case_percentage, 100_00);
        assert!(ft.is_ok());
        let ft = ft.unwrap();
        assert_eq!(ft.member, "peter");
        assert_eq!(ft.amount.unwrap(), 10_00);


        // invalid cases
        let case_err_nosplit = "peter25,22";
        let ft = Target::parse(case_err_nosplit, 100_00);
        assert!(ft.is_err());
        let case_err_noamount = "peter";
        let ft = Target::parse(case_err_noamount, 100_00);
        assert!(ft.is_ok());
        let case_err_noname = "25,22";
        let ft = Target::parse(case_err_noname, 100_00);
        assert!(ft.is_err());
        let case_err_noamount_percentage = "peter:%";
        let ft = Target::parse(case_err_noamount_percentage, 100_00);
        assert!(ft.is_err());
        let case_err_nothing = ":";
        let ft = Target::parse(case_err_nothing, 100_00);
        assert!(ft.is_err());
        let case_err_noname_double = ":25,22";
        let ft = Target::parse(case_err_noname_double, 100_00);
        assert!(ft.is_err());
        let case_err_noamount_double = "peter:";
        let ft = Target::parse(case_err_noamount_double, 100_00);
        assert!(ft.is_err());
    }
}


#[derive(Debug, PartialEq)]
pub(crate) struct Transaction {
    from: String,
    to: String,
    amount: Money,
}

impl Transaction {
    pub(crate) fn new(from: &str, to: &str, amount: Money) -> Self {
        Transaction {
            from: from.to_string(),
            to: to.to_string(),
            amount: amount.abs(),
        }
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} \tpays {}:\t{:.02}€", self.from, self.to, self.amount as f32 / 100.)
    }
}

pub struct Splitter {
    state: SplitterState,
    db_path: PathBuf,
}

impl Splitter {
    pub(crate) const CURRENT_VERSION: &'static str = "0.1.0";
    pub(crate) const NAME_REGEX: &'static str = r"^[a-zA-Z0-9][a-zA-Z0-9_\-()]*$";

    pub fn new(source: PathBuf) -> Self {
        let state = SplitterState::new(source.clone());
        Self {
            state,
            db_path: source,
        }
    }

    fn confirm() -> bool {
        println!("Confirm? [yY|nN]: ");
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer).expect("stdin Input Error");
        if buffer.starts_with(['y', 'Y']) {
            sleep(Duration::from_secs(2));
            true
        } else {
            false
        }
    }
    fn balance(&mut self, group: Option<String>) -> Result<()> {
        let group = self.state.get_group_mut(group)?;
        let mut transactions = group.balance();
        println!("The following transactions are recommended:");
        for t in &transactions {
            println!("{}", t);
        }
        if Self::confirm() {
            let tac_len = transactions.len();
            let tac =
                transactions.drain(..)
                    .fold(HashMap::with_capacity(tac_len),
                          |mut accu, ta| {
                              accu.insert(ta.from, ta.amount);
                              accu.insert(ta.to, -ta.amount);
                              accu
                          },
                    );
            group.apply_tachange(tac);
        }
        Ok(())
    }

    pub(crate) fn run(&mut self, command: SubCommand) -> Result<()> {
        match command {
            SubCommand::Add { group, members } => {
                let group = self.state.get_group_mut(group)?;
                group.add(members)?;
            }
            SubCommand::Remove { group, force, members } => {
                let group = self.state.get_group_mut(group)?;
                group.remove(members, force.unwrap_or(false))?;
            }
            SubCommand::Create { name, members } => {
                if !Regex::new(Splitter::NAME_REGEX).unwrap().is_match(name.as_str()) {
                    return Err(anyhow!(SplitterError::InvalidName))
                        .context(name);
                }
                if self.state.groups.iter().any(|thing| thing.name == name) {
                    return Err(anyhow!(SplitterError::InvalidName))
                        .context(format!("Group already exists! {}", name));
                }
                self.state.groups.push(Group::new(name, members, None)?);
                self.state.current_group = Some(self.state.groups.len() - 1);
            }
            SubCommand::Undo { group, index } => {
                let group = self.state.get_group_mut(group)?;
                let (lentry, currency) = (group.get_log(index)?, group.currency);
                println!("You are about to undo\n`{}`", lentry.to_string(currency));
                println!("This cannot be reversed");
                group.apply_tachange(lentry.reversed_change());
                group.remove_log(index)?;
                println!("Success");
            }
            SubCommand::DeleteGroup { group, yes } =>
                self.state.delete_group(group, yes.unwrap_or(false))?,
            SubCommand::List { group, all } => {
                if all.unwrap_or(false) {
                    for g in &self.state.groups {
                        println!("{}\n", g.list());
                    }
                } else {
                    let gidx = self.state.get_group_idx(group.clone())?;
                    let group = self.state.get_group(group)?;
                    println!("\n{}\n", group.list());
                    self.state.current_group = Some(gidx);
                }
            }
            SubCommand::Stat { group, all } => {
                if all.unwrap_or(false) {
                    for g in &self.state.groups {
                        println!("{}\n", g.stat());
                    }
                } else {
                    let gidx = self.state.get_group_idx(group.clone())?;
                    let group = self.state.get_group(group)?;
                    println!("{}", group.stat());
                    self.state.current_group = Some(gidx);
                }
            }
            SubCommand::Pay { amount, group, from, to } =>
                {
                    let gidx = self.state.get_group_idx(group.clone())?;
                    let group = self.state.get_group_mut(group)?;
                    group.log_pay_transaction(
                        (amount * group.currency.subdivision()) as Money,
                        from,
                        to,
                    )?;
                    self.state.current_group = Some(gidx);
                }
            SubCommand::Split {
                amount,
                group,
                from,
                to,
                name,
                balance_rest,
            } => {
                let gidx = self.state.get_group_idx(group.clone())?;
                let group = self.state.get_group_mut(group)?;
                group.split((amount * 100.) as i64, from, to, name,
                            balance_rest.unwrap_or(false))?;
                self.state.current_group = Some(gidx);
            }
            SubCommand::Balance { group } => {
                let gidx = self.state.get_group_idx(Some(group.clone()))?;
                self.balance(Some(group))?;
                self.state.current_group = Some(gidx);
            }
        };
        Ok(())
    }

    fn compress(input: String) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut com_rdr =
            CompressorReader::new(input.as_bytes(), 4096, 6, 22);
        com_rdr.read_to_end(&mut compressed).unwrap();
        compressed
    }

    fn decompress(input: &[u8]) -> String {
        let mut decompressor = Decompressor::new(input, 4096);
        let mut dec_data = String::new();
        decompressor.read_to_string(&mut dec_data).unwrap();
        dec_data
    }

    pub(crate) fn save(&self) -> Result<()> {
        let raw = serde_yaml::to_string(&self.state)?;
        let result = Self::compress(raw);
        let mut file = std::fs::File::create(self.db_path.as_path())?;
        file.write_all(result.as_slice())?;
        Ok(())
    }
}
