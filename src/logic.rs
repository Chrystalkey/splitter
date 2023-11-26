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
use crate::error::InternalSplitterError;
use crate::logging::{LogEntry, LoggedCommand};

pub(crate) type TransactionChange = HashMap<String, i64>;

#[derive(Serialize, Deserialize, Clone)]
struct Member {
    name: String,
    balance: i64,
}

impl Member {
    fn new(name: String) -> Self {
        Member {
            name,
            balance: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Transaction {
    from: String,
    to: String,
    amount: i64,
}

impl Transaction {
    fn new(from: &String, to: &String, amount: i64) -> Self {
        Transaction {
            from: from.clone(),
            to: to.clone(),
            amount: amount.abs(),
        }
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} \tpays {}:\t{:.02}€", self.from, self.to, self.amount as f32 / 100.)
    }
}

#[derive(Serialize, Deserialize)]
struct Group {
    name: String,
    members: Vec<Member>,
    log: Vec<LogEntry>,
}

impl Group {
    fn new(name: String, members: Vec<String>) -> Self {
        let membrs = {
            let mut vec = Vec::with_capacity(members.len());
            for m in members {
                assert!(Regex::new(Logic::NAME_REGEX).unwrap().is_match(m.as_str()),
                        "Name {} is not allowed for members", m);
                vec.push(Member::new(m));
            }
            vec
        };
        Self {
            name,
            members: membrs,
            log: vec![],
        }
    }
    fn stat(&self) -> String {
        let mut string =
            format!("Group Statistics for group {}:\n\
        Members:\n\
        ", self.name);
        for mem in &self.members {
            string = format!("{}\n{}: {:.02}€", string, mem.name, mem.balance as f32 / 100.);
        }
        string
    }
    fn list(&self) -> String {
        let accu = format!("Log Listing for Group {}\n", self.name);
        self.log.iter()
            .fold(accu, |a, e| format!("{a}{e}\n"))
    }
}

#[derive(Serialize, Deserialize)]
struct SplitterState {
    groups: Vec<Group>,
    current_group: Option<usize>,
}

impl SplitterState {
    fn new(source: PathBuf) -> Self {
        return if source.exists() {
            if source.is_file() {
                let raw = std::fs::read(source).unwrap();
                serde_yaml::from_str(
                    Logic::decompress(raw.as_slice()).as_str()
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
                groups: vec![],
                current_group: None,
            }
        };
    }

    /// get a reference to the group or panic
    fn get_group(&self, group_name: Option<String>) -> &Group {
        let group = match group_name {
            None => (self.groups.get(self.current_group.unwrap_or(0)))
                .expect("Error: No group was found"),
            Some(name) => self.groups.iter().find(|&gn| gn.name == name)
                .expect("Error: Could not find a group with this name")
        };
        group
    }
    fn get_group_mut(&mut self, group_name: Option<String>) -> &mut Group {
        let group = match group_name {
            None => (self.groups.get_mut(self.current_group.unwrap_or(0)))
                .expect("Error: No group was found"),
            Some(name) => self.groups.iter_mut().find(|gn| gn.name == name)
                .expect("Error: Could not find a group with this name")
        };
        group
    }
}

/// helper struct containing money and a name. Can be used as a "from" or as a "to"
/// Can be parsed from --from/to {name}[:amount[%]]
#[derive(PartialEq, Serialize, Deserialize)]
pub(crate) struct Target {
    pub(crate) member: String,
    pub(crate) amount: Option<i64>,
}

impl Target {
    /// Parses a target directive specified via `--from` or `--to` into a Target Struct
    fn parse(input: &str, total_money: i64) -> Result<Self, InternalSplitterError> {
        let in_split: Vec<_> = input.trim_end_matches('%').split(':').collect();
        if in_split[0].is_empty() {
            return Err(InternalSplitterError::InvalidFormat(
                "Please use the format <name>:[<number>[%]]. (maybe you forgot ':'?".to_string()));
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
            if !Regex::new(Logic::NAME_REGEX).unwrap().is_match(in_split[0]) {
                return Err(InternalSplitterError::InvalidName(format!("Name {} is not valid.", in_split[0])));
            }
            Ok(Self {
                member: in_split[0].to_owned(),
                amount: None,
            })
        } else {
            Err(InternalSplitterError::InvalidFormat(
                "Please use the format <name>:[<number>[%]]. (maybe you forgot ':'?".to_string()))
        }
    }
    /// Parses entries that originate with --from or --to arguments.
    /// it returns the list of the names together with an option denoting their amounts.
    /// None means they did not specify an amount.
    /// The second return value is the total amount that was explicitly given
    /// The third return value is the number of wildcard givers
    fn parse_multiple(raw_targets: Vec<String>, total_amount: i64) -> Result<(Vec<Target>, i64, usize), InternalSplitterError> {
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
            return Err(InternalSplitterError::InvalidSemantic(
                format!("Error: The amounts specified with '--from' or '--to' sum up to more than the total amount: {} vs {}",
                        summed, total_amount)
            ));
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

pub struct Logic {
    state: SplitterState,
    db_path: PathBuf,
}

impl Logic {
    const NAME_REGEX: &'static str = r"^[a-zA-Z0-9][a-zA-Z0-9_\-()]*$";
    pub fn new(source: PathBuf) -> Self {
        let state = SplitterState::new(source.clone());
        Self {
            state,
            db_path: source,
        }
    }

    fn create_group(&mut self, name: String, members: Vec<String>) {
        assert!(Regex::new(Self::NAME_REGEX).unwrap().is_match(name.as_str()),
                "Please only use alphanumeric names and _-(), starting with one of the former");
        self.state.groups.push(Group::new(name, members));
    }

    fn stat(&self, group_name: Option<String>, all: bool) {
        if all {
            for g in &self.state.groups {
                println!("\n{}\n", g.stat());
            }
        } else {
            let group = self.state.get_group(group_name);
            println!("\n{}\n", group.stat());
        }
    }
    fn list(&self, group_name: Option<String>, all: bool) {
        if all {
            for g in &self.state.groups {
                println!("\n{}\n", g.list());
            }
        } else {
            let group = self.state.get_group(group_name);
            println!("\n{}\n", group.list());
        }
    }

    /// Helper function to split `cents` Cents among `among` many people as just a possible
    /// This means splitting as equal as possible, distributing leftover cents from the top equally
    fn split_equal_among(cents: i64, among: usize) -> Vec<i64> {
        let mut result = Vec::with_capacity(among);
        let everyone_split = cents / among as i64;

        let (neg, cents) = if cents < 0 { (true, -cents) } else { (false, cents) };

        let remainder = cents as u64 % among as u64;
        let mut remainder = if neg { -(remainder as i64) } else { remainder as i64 };

        result.resize_with(result.capacity(), || everyone_split);
        for res in result.iter_mut() {
            *res += remainder.signum();
            remainder -= remainder.signum();
            if remainder.abs() == 0 {
                break;
            }
        }
        result
    }

    /// split endpoint calling the calculation function, logging the result and applying the result to
    /// the current member's balances
    fn split(&mut self, amount: i64, group_name: Option<String>,
             from: Vec<String>, to: Vec<String>, name: String, balance_rest: bool)
    {
        let group = self.state.get_group(group_name.clone());

        let (transaction, from, to) =
            Self::split_into_transaction(amount, group, from.clone(), to.clone(), balance_rest)
                .unwrap_or_else(|error| panic!("Transaction Split was not Successful:\n{:?}", error));
        // log the transaction about to take place
        let group = self.state.get_group_mut(group_name);
        group.log.push(LogEntry::new(
            LoggedCommand::Split {
                amount,
                from,
                to,
                name,
                group: group.name.clone(),
                balance_rest,
            },
            transaction.clone(),
        ));
        // set values according to the transaction bin
        for member in &mut group.members {
            member.balance += transaction.get(member.name.as_str()).unwrap();
        }
    }

    /// receives vectors of --from and --to arguments, a amount to be split, a group name this
    /// should be assigned to and a flag indicating whether members named in a --to directive
    /// should share the rest of the bill with them
    fn split_into_transaction(total_amount: i64, group: &Group,
                              from: Vec<String>, to: Vec<String>, balance_rest: bool)
                              -> Result<(TransactionChange, Vec<Target>, Vec<Target>), InternalSplitterError> {
        let givers = Target::parse_multiple(from, total_amount)?;
        let recvrs = Target::parse_multiple(to, total_amount)?;
        if recvrs.0.iter().any(|el| el.amount.is_none()) {
            return Err(InternalSplitterError::InvalidFormat("Amounts for --to must be specified explicitly".to_string()));
        }
        // normalize givers to contain entries for all members of the group
        let moneysplit =
            Self::split_equal_among(total_amount - givers.1, givers.2);
        let mut wcg_index = 0;
        let mut transaction_map = HashMap::with_capacity(group.members.len());

        // positively add all the froms
        for mem in &group.members {
            if let Some(giver) = givers.0.iter().find(|&target| target.member == mem.name)
            {
                if let Some(amount) = giver.amount {
                    transaction_map.insert(mem.name.clone(), amount);
                } else {
                    transaction_map.insert(mem.name.clone(), moneysplit[wcg_index]);
                    wcg_index += 1;
                }
            } else {
                transaction_map.insert(mem.name.clone(), 0);
            }
        }

        // subtract all tos from the balance of the transaction
        // peter started with 0, but takes 5€ of the pot, reaching a balance of -5€
        // if balance_rest is true, everything gets split onto the --to takers as well, if not, they
        // are excluded from the calculation and pay exactly as much as specified
        let moneysplit = Self::split_equal_among(
            total_amount - recvrs.1,
            group.members.len() - if balance_rest { 0 } else { recvrs.0.len() },
        );
        let mut ms_idx = 0;
        for mem in &group.members {
            if let Some(recv) = recvrs.0.iter().find(|&el| el.member == mem.name) {
                let x = transaction_map.get_mut(&mem.name).unwrap();
                *x -= recv.amount.unwrap();
                if balance_rest {
                    *x -= moneysplit[ms_idx];
                    ms_idx += 1;
                }
            } else {
                let x = transaction_map.get_mut(&mem.name).unwrap();
                *x -= moneysplit[ms_idx];
                ms_idx += 1;
            }
        }
        // if balance rest is not specified, balance between the non-specified group members
        Ok((transaction_map, givers.0, recvrs.0))
    }
    fn balance_group(group: &Group) -> Vec<Transaction> {
        let members = &group.members;
        let mut creditors: Vec<Member> =
            members.iter().filter(|&mem| mem.balance > 0).cloned().collect();
        let mut debtors: Vec<Member> =
            members.iter().filter(|&mem| mem.balance < 0).cloned().collect();
        creditors.sort_unstable_by(|el1, el2| el1.balance.partial_cmp(&el2.balance).unwrap());
        debtors.sort_unstable_by(
            |el1, el2| el1.balance.abs().partial_cmp(&el2.balance.abs())
                .unwrap());
        let mut transactions = vec![];
        // find matching c and d & match them up
        for d in debtors.iter_mut() {
            for c in creditors.iter_mut() {
                if -d.balance < c.balance {
                    break; // break the loop
                }
                if d.balance == -c.balance {
                    transactions.push(Transaction::new(&d.name, &c.name, c.balance));
                    d.balance = 0;
                    c.balance = 0;
                }
            }
        }

        let mut c_idx = 0;
        // non-matching loop
        for d in debtors.iter_mut() {
            if d.balance == 0 {
                continue;
            }
            while creditors.get(c_idx).unwrap().balance == 0 {
                c_idx += 1;
            }
            let mut c = creditors.get_mut(c_idx).unwrap();
            if c.balance == -d.balance {
                transactions.push(Transaction::new(&d.name, &c.name, c.balance));
                d.balance = 0;
                c.balance = 0;
                c_idx += 1;
                continue;
            }
            while c.balance < -d.balance {
                d.balance += c.balance;
                transactions.push(Transaction::new(&d.name, &c.name, c.balance));
                c.balance = 0;
                c_idx += 1;
                c = creditors.get_mut(c_idx).unwrap();
            }
            if c.balance > -d.balance {
                c.balance += d.balance;
                transactions.push(Transaction::new(&d.name, &c.name, d.balance));
                d.balance = 0;
            }
        }
        transactions
    }
    fn confirm() -> bool {
        println!("Confirm? [yY|nN]: ");
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer).expect("stdin Input Error");
        if buffer.starts_with(['y', 'Y']) {
            sleep(Duration::from_secs(2));
            return true;
        } else {
            return false;
        }
    }
    fn balance(&mut self, group: String) {
        let group = self.state.get_group_mut(Some(group));
        let transactions = Self::balance_group(group);
        println!("The following transactions are recommended:");
        for t in &transactions {
            println!("{}", t);
        }
        if Self::confirm() {
            for ta in transactions {
                // bad, optimize later (todo)
                let d = group.members.iter_mut().find(|m| m.name == ta.from).unwrap();
                d.balance += ta.amount;
                let c = group.members.iter_mut().find(|m| m.name == ta.to).unwrap();
                c.balance -= ta.amount;
                group.log.push(LogEntry::new(
                    LoggedCommand::Pay {
                        amount: ta.amount,
                        from: ta.from.clone(),
                        to: ta.to.clone(),
                    }, HashMap::from([(ta.from, -ta.amount), (ta.to, ta.amount)])));
            }
        }
    }
    fn pay(&mut self, amount: i64, group: Option<String>, from: String, to: String) {
        let group = self.state.get_group_mut(group);
        // calculate transaction
        let mut transaction = HashMap::with_capacity(2);
        transaction.insert(from.clone(), -amount);
        transaction.insert(to.clone(), amount);

        // apply transaction
        let mut found_both = 0;
        for m in &mut group.members {
            if m.name == from {
                found_both += 1;
                m.balance += amount;
            } else if m.name == to {
                found_both += 1;
                m.balance -= amount;
            }
            if found_both == 2 {
                break;
            }
        }

        // log transaction
        group.log.push(
            LogEntry::new(LoggedCommand::Pay { amount, from, to },
                          transaction)
        );
    }
    fn delete_group(&mut self, group_name: String, yes: bool) {
        println!("This will delete the group '{}' forever with no more undo options available.\n",
                 group_name);
        self.stat(Some(group_name.clone()), false);
        let really = yes || Self::confirm();
        if really && !yes {
            println!("Confirmed. Deleting group");
            self.state.groups = self.state.groups.drain(..)
                .filter(|grp| grp.name != group_name).collect();
            self.state.current_group = Some(0);
        } else if yes {
            self.state.groups = self.state.groups.drain(..)
                .filter(|grp| grp.name != group_name).collect();
            self.state.current_group = Some(0);
        } else if !yes {
            println!("Operation Cancelled");
        }
    }
    pub(crate) fn run(&mut self, command: SubCommand) {
        match command {
            SubCommand::Create { name, members } => self.create_group(name, members),
            SubCommand::Undo { group, index } => {
                let _ = (group, index);
                todo!("Undo is not implemented")
            }
            SubCommand::DeleteGroup { group, yes } => self.delete_group(group, yes.unwrap_or(false)),
            SubCommand::List { group, all } => self.list(group, all.unwrap_or(false)),
            SubCommand::Stat { group, all } => self.stat(group, all.unwrap_or(false)),
            SubCommand::Pay { amount, group, from, to } =>
                self.pay((amount * 100.) as i64, group, from, to),
            SubCommand::Split {
                amount, group, from, to, name, balance_rest
            } => self.split((amount * 100.) as i64, group, from, to, name,
                            balance_rest.unwrap_or(false)),
            SubCommand::Balance { group } => self.balance(group),
        };
    }
    fn compress(input: String) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut com_rdr =
            CompressorReader::new(input.as_bytes(), 4096, 6, 22);
        com_rdr.read_to_end(&mut compressed).unwrap();
        return compressed;
    }
    fn decompress(input: &[u8]) -> String {
        let mut decompressor = Decompressor::new(input, 4096);
        let mut dec_data = String::new();
        decompressor.read_to_string(&mut dec_data).unwrap();
        return dec_data;
    }

    pub(crate) fn save(&self) -> Result<(), InternalSplitterError> {
        let raw = serde_yaml::to_string(&self.state)?;
        let result = Self::compress(raw);
        let mut file = std::fs::File::create(self.db_path.as_path())?;
        file.write_all(result.as_slice())?;
        Ok(())
    }
}

#[cfg(test)]
mod logic_tests {
    use super::*;

    #[test]
    fn test_balance_equal() {
        let mut group =
            Group::new("testgroup".to_string(),
                       vec!["Alice".to_string(), "Bob".to_string()]);
        group.members[0].balance = -10_00;
        group.members[1].balance = 10_00;
        let tas = Logic::balance_group(&group);
        assert_eq!(tas.len(), 1);
        assert_eq!(tas[0].from, "Alice");
        assert_eq!(tas[0].to, "Bob");
        assert_eq!(tas[0].amount, 10_00);
    }

    #[test]
    fn test_balance_unequal() {
        let mut group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(),
                            "Charly".to_string(), "Django".to_string()]);
        group.members[0].balance = -1685;
        group.members[1].balance = 316;
        group.members[2].balance = 2117;
        group.members[3].balance = -748;
        let tas = Logic::balance_group(&group);
        assert_eq!(tas[0], Transaction::new(&"Django".to_string(), &"Bob".to_string(), 316));
        assert_eq!(tas[1], Transaction::new(&"Django".to_string(), &"Charly".to_string(), 432));
        assert_eq!(tas[2], Transaction::new(&"Alice".to_string(), &"Charly".to_string(), 1685));
    }

    #[test]
    fn test_delete_group_success() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(), "Charly".to_string()]);
        let mut splitter = Logic {
            state: SplitterState {
                groups: vec![group],
                current_group: Some(0),
            },
            db_path: "".into(),
        };
        assert_eq!(splitter.state.groups.len(), 1);
        splitter.delete_group("testgroup".to_string(), true);
        assert_eq!(splitter.state.groups.len(), 0);
    }

    #[test]
    #[should_panic]
    fn test_delete_group_failure() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(), "Charly".to_string()]);
        let mut splitter = Logic {
            state: SplitterState {
                groups: vec![group],
                current_group: Some(0),
            },
            db_path: "".into(),
        };
        assert_eq!(splitter.state.groups.len(), 1);
        splitter.delete_group("txt".to_string(), true);
    }

    #[test]
    fn test_split_equal_among() {
        // test positive values
        // test "perfect" split
        let result = Logic::split_equal_among(100, 10);
        for x in result {
            assert_eq!(x, 10);
        }
        // test "imperfect" split
        let result = Logic::split_equal_among(100, 9);
        let expected_vec = vec![12, 11, 11, 11, 11, 11, 11, 11, 11];
        for i in 0..expected_vec.len() {
            assert_eq!(result[i], expected_vec[i]);
        }
        // test negative values
        // test "perfect" split
        let result = Logic::split_equal_among(-100, 10);
        for x in result {
            assert_eq!(x, -10);
        }
        // test "imperfect" split
        let result = Logic::split_equal_among(-100, 9);
        let expected_vec = vec![-12, -11, -11, -11, -11, -11, -11, -11, -11];
        for i in 0..expected_vec.len() {
            assert_eq!(result[i], expected_vec[i]);
        }
    }

    #[test]
    fn test_parse_targets() {
        let from_entries = vec!["alice:12".to_string(), "bob:13".to_string(), "charly:10%".to_string()];

        let parsed = Target::parse_multiple(from_entries, 100_00);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.0.len(), 3);
        assert!(parsed.0.contains(&Target { member: "alice".to_string(), amount: Some(12_00) }), "Alice missing");
        assert!(parsed.0.contains(&Target { member: "bob".to_string(), amount: Some(13_00) }), "Bob missing");
        assert!(parsed.0.contains(&Target { member: "charly".to_string(), amount: Some(10_00) }), "Charly missing");
        assert_eq!(parsed.1, 35_00, "Summed amount is not correct");
        assert_eq!(parsed.2, 0, "No Members had unspecified amounts");

        // two wildcard givers
        let from_entries = vec!["alice:12".to_string(), "bob".to_string(), "charly".to_string()];

        let parsed = Target::parse_multiple(from_entries, 100_00);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.0.len(), 3);
        assert!(parsed.0.contains(&Target { member: "alice".to_string(), amount: Some(12_00) }), "Alice missing");
        assert!(parsed.0.contains(&Target { member: "bob".to_string(), amount: None }), "Bob missing");
        assert!(parsed.0.contains(&Target { member: "charly".to_string(), amount: None }), "Charly missing");
        assert_eq!(parsed.1, 12_00, "Summed amount is not correct");
        assert_eq!(parsed.2, 2, "No Members had unspecified amounts");

        // froms > 100%
        let from_entries = vec!["alice:90".to_string(), "bob:20".to_string(), "charly:10%".to_string()];

        let parsed = Target::parse_multiple(from_entries, 100_00);
        assert!(parsed.is_err(), "Expected error showing incorrect amounts");
    }

    #[test]
    fn test_simple_split_one_giver() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(), "Charly".to_string()]);

        let transaction_bins = Logic::split_into_transaction(
            120, &group, vec!["Alice".to_string()], vec![], false);
        // alle - 120/3 = -40, Alice +120 | A80, B-40,c-40
        assert!(transaction_bins.is_ok());
        let (transaction_bins, _, _) = transaction_bins.unwrap();
        assert_eq!(transaction_bins.len(), 3);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert_eq!(transaction_bins["Alice"], 80);
        assert_eq!(transaction_bins["Bob"], -40);
        assert_eq!(transaction_bins["Charly"], -40);
    }

    #[test]
    fn test_multiple_givers() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(),
                            "Charly".to_string(), "Django".to_string()]);

        let transaction_bins = Logic::split_into_transaction(
            120, &group,
            vec!["Alice".to_string(), "Bob".to_string()], vec![], false);
        // alle - 120/4 = -30, Alice +60, Bob +60 | A30, B30, C-30, D-30
        assert!(transaction_bins.is_ok());
        let (transaction_bins, _, _) = transaction_bins.unwrap();
        assert_eq!(transaction_bins.len(), 4);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert!(transaction_bins.contains_key("Django"));
        assert_eq!(transaction_bins["Alice"], 30);
        assert_eq!(transaction_bins["Bob"], 30);
        assert_eq!(transaction_bins["Charly"], -30);
        assert_eq!(transaction_bins["Django"], -30);
    }

    #[test]
    fn test_one_to() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(),
                            "Charly".to_string(), "Django".to_string()]);

        let transaction_bins = Logic::split_into_transaction(
            130, &group,
            vec!["Bob".to_string()],
            vec!["Alice:0,1".to_string()], false);
        // alice - 10 -> A-10
        // total-10 = 120
        // BCD - 120/3 = -40
        // Bob + 130
        // A-10, B-40+130=90, C-40, D-40
        assert!(transaction_bins.is_ok());
        let (transaction_bins, _, _) = transaction_bins.unwrap();
        assert_eq!(transaction_bins.len(), 4);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert!(transaction_bins.contains_key("Django"));
        assert_eq!(transaction_bins["Alice"], -10);
        assert_eq!(transaction_bins["Bob"], 90);
        assert_eq!(transaction_bins["Charly"], -40);
        assert_eq!(transaction_bins["Django"], -40);
    }

    #[test]
    fn test_multiple_to() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(),
                            "Charly".to_string(), "Django".to_string()]);

        let transaction_bins = Logic::split_into_transaction(
            140, &group,
            vec!["Bob".to_string()],
            vec!["Alice:0,1".to_string(), "Charly:0.1".to_string()], false);
        // alice - 10 -> A-10
        // charly -10 -> C-10
        // total-10-10 = 120
        // BD - 120/2 = -60
        // Bob + 140
        // A-10, B-60+140=80, C-10, D-60
        assert!(transaction_bins.is_ok());
        let (transaction_bins, _, _) = transaction_bins.unwrap();
        assert_eq!(transaction_bins.len(), 4);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert!(transaction_bins.contains_key("Django"));
        assert_eq!(transaction_bins["Alice"], -10);
        assert_eq!(transaction_bins["Bob"], 80);
        assert_eq!(transaction_bins["Charly"], -10);
        assert_eq!(transaction_bins["Django"], -60);
    }

    #[test]
    fn test_balance_rest() {
        let group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(),
                            "Charly".to_string(), "Django".to_string()]);

        let transaction_bins = Logic::split_into_transaction(
            140, &group,
            vec!["Bob".to_string()],
            vec!["Alice:0,1".to_string(), "Charly:0.1".to_string()], true);
        // alice - 10 -> A-10
        // charly -10 -> C-10
        // total-10-10 = 120
        // ABCD - 120/4 = -30
        // Bob + 140
        // A-10-30=-40, B+140-30=110, C-10-30=-40, D-30
        assert!(transaction_bins.is_ok());
        let (transaction_bins, _, _) = transaction_bins.unwrap();
        assert_eq!(transaction_bins.len(), 4);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert!(transaction_bins.contains_key("Django"));
        assert_eq!(transaction_bins["Alice"], -40);
        assert_eq!(transaction_bins["Bob"], 110);
        assert_eq!(transaction_bins["Charly"], -40);
        assert_eq!(transaction_bins["Django"], -30);
    }
}