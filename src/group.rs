use std::collections::{HashMap, HashSet};
use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::error::InternalSplitterError;
use crate::logging::{LogEntry, LoggedCommand};
use crate::logging::LoggedCommand::Split;
use crate::logic::{Money, Splitter, Target, Transaction, TransactionChange};
use crate::money::Currency;

#[derive(Serialize, Deserialize)]
pub(crate) struct Group {
    pub(crate) name: String,
    pub(crate) currency: Currency,
    members: HashMap<String, Money>,
    log: Vec<LogEntry>,
}

impl Group {
    pub(crate) fn new(name: String, members: Vec<String>, currency: Option<Currency>) -> Result<Self, InternalSplitterError> {
        if members.is_empty() {
            return Err(InternalSplitterError::InvalidSemantic("Group must have at least one member".into()));
        }
        let mut membrs = {
            let mut map = HashMap::with_capacity(members.len());
            for m in members {
                assert!(Regex::new(Splitter::NAME_REGEX).unwrap().is_match(m.as_str()),
                        "Name {} is not allowed for members", m);
                map.insert(m, 0);
            }
            map
        };
        Ok(Self {
            name,
            currency: currency.unwrap_or(Currency::EUR),
            members: membrs,
            log: vec![],
        })
    }
    pub(crate) fn get_log(&self, index: Option<usize>) -> Result<&LogEntry, InternalSplitterError> {
        if self.log.is_empty() {
            return Err(InternalSplitterError::LogEntryNotFound);
        }
        let index = index.unwrap_or(self.log.len() - 1);
        self.log.get(index).ok_or(InternalSplitterError::LogEntryNotFound)
    }
    pub(crate) fn remove_log(&mut self, index: Option<usize>) -> Result<LogEntry, InternalSplitterError> {
        if self.log.is_empty() {
            return Err(InternalSplitterError::LogEntryNotFound);
        }
        let index = index.unwrap_or(self.log.len() - 1);
        if index >= self.log.len() {
            return Err(InternalSplitterError::LogEntryNotFound);
        }
        Ok(self.log.remove(index))
    }
    pub(crate) fn stat(&self) -> String {
        let mut string =
            format!("Group Statistics for group {} ({}):\n\
        Members:\n\
        ", self.name, self.currency);

        for (name, balance) in &self.members {
            string = format!("{}\n{}: {:.02}{}", string, name, *balance as f32 / self.currency.subdivision(), self.currency);
        }
        string
    }
    pub(crate) fn list(&self) -> String {
        let accu = format!("Log Listing for Group {} ({})\n", self.name, self.currency);
        self.log.iter()
            .fold(accu, |a, e| format!("{a}{}\n", e.to_string(self.currency)))
    }
    pub(crate) fn balance(&self) -> Vec<Transaction> {
        let members = &self.members;
        struct Member {
            name: String,
            balance: Money,
        }
        let mut creditors: Vec<Member> =
            members.iter().filter(|&(_, balance)| *balance > 0)
                .map(|(name, balance)| Member { name: name.clone(), balance: *balance })
                .collect();
        let mut debtors: Vec<Member> =
            members.iter().filter(|&(_, balance)| *balance < 0)
                .map(|(name, balance)| Member { name: name.clone(), balance: *balance })
                .collect();
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
    pub(crate) fn add(&mut self, mut members: Vec<String>) -> Result<(), InternalSplitterError> {
        let mut duplicates = vec![];
        let mut errors = vec![];
        for member in members.drain(..) {
            if self.members.contains_key(&member) {
                duplicates.push(member);
            } else if !Regex::new(Splitter::NAME_REGEX)
                .unwrap().is_match(member.as_str()) {
                errors.push(member);
            } else {
                self.members.insert(member, 0);
            }
        }
        if duplicates.is_empty() && errors.is_empty() {
            Ok(())
        } else {
            Err(InternalSplitterError::InvalidName(
                format!("duplicates: {:#?}\ninvalid names: {:#?}", duplicates, errors)))
        }
    }
    pub(crate) fn remove(&mut self, mut members: Vec<String>, force: bool) -> Result<(), InternalSplitterError> {
        let mut errors = vec![];
        for member in members.drain(..) {
            if !self.members.contains_key(&member) ||
                (*self.members.get(&member).unwrap() != 0 && !force) {
                errors.push(member);
            } else {
                self.members.remove(&member);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(InternalSplitterError::InvalidName(
                format!("Could not remove some members: Probably they either have to pay money, get money,\
                 or they do not appear in the list:\n{:?}", errors)
            ))
        }
    }
    pub(crate) fn apply_tachange(&mut self, tac: TransactionChange) {
        for (name, balance) in self.members.iter_mut() {
            *balance += tac.get(name.as_str()).unwrap();
        }
    }
    pub(crate) fn log_pay_transaction(&mut self, amount: i64, from: String, to: String) -> Result<(), InternalSplitterError> {
        // calculate transaction
        let mut transaction = HashMap::with_capacity(2);
        transaction.insert(from.clone(), -amount);
        transaction.insert(to.clone(), amount);

        // apply transaction
        let mut found_both = 0;
        for (name, balance) in self.members.iter_mut() {
            if name == &from {
                found_both += 1;
                *balance += amount;
            } else if name == &to {
                found_both += 1;
                *balance -= amount;
            }
            if found_both == 2 {
                break;
            }
        }
        if found_both != 2 {
            return Err(InternalSplitterError::MemberNotFound(format!("Either {from} or {to} do not exists within this group")));
        }

        // log transaction
        self.log.push(
            LogEntry::new(LoggedCommand::Pay { amount, from, to },
                          transaction)
        );
        Ok(())
    }
    /// split endpoint calling the calculation function, logging the result and applying the result to
    /// the current member's balances
    pub(crate) fn split(&mut self, amount: Money,
                        from: Vec<String>, to: Vec<String>, name: String, balance_rest: bool)
                        -> Result<(), InternalSplitterError>
    {
        let (transaction, from, to) =
            split_into_transaction(amount, self, from.clone(), to.clone(), balance_rest)?;
        // log the transaction about to take place
        self.log.push(LogEntry::new(
            LoggedCommand::Split {
                amount,
                from,
                to,
                name,
                group: self.name.clone(),
                balance_rest,
            },
            transaction.clone(),
        ));
        // set values according to the transaction bin
        for (name, balance) in &mut self.members {
            *balance += *transaction.get(name.as_str()).unwrap();
        }
        Ok(())
    }
}


/// Helper function to split `cents` Cents among `among` many people as just a possible. among > 0.
/// This means splitting as equal as possible, distributing leftover cents from the top equally
fn split_equal_among(cents: Money, among: usize) -> Vec<Money> {
    let mut result = Vec::with_capacity(among);
    let everyone_split = cents / among as Money;

    let neg = cents < 0;
    let cents = cents.unsigned_abs() as usize;

    let remainder = cents % among;
    let mut remainder = if neg { -(remainder as Money) } else { remainder as Money };

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


/// receives vectors of --from and --to arguments, a amount to be split, a group name this
/// should be assigned to and a flag indicating whether members named in a --to directive
/// should share the rest of the bill with them
fn split_into_transaction(total_amount: Money, group: &Group,
                          from: Vec<String>, to: Vec<String>, balance_rest: bool)
                          -> Result<(TransactionChange, Vec<Target>, Vec<Target>), InternalSplitterError> {
    let givers = Target::parse_multiple(from, total_amount)?;
    let recvrs = Target::parse_multiple(to, total_amount)?;
    if recvrs.0.iter().any(|el| el.amount.is_none()) {
        return Err(InternalSplitterError::InvalidTargetFormat("Amounts for --to must be specified explicitly".to_string()));
    } else if givers.0.iter().fold(0i64, |a, b| a.saturating_add(b.amount.unwrap_or(i64::MAX))) <=
        recvrs.0.iter().fold(0, |a, b| b.amount.unwrap() + a) {
        return Err(InternalSplitterError::InvalidSemantic(
            "Amounts of --from directives must either contain a catch-all or be >= amounts specified by --to".to_string()
        ));
    } else if
    recvrs.0.iter().any(|el| !group.members.contains_key(&el.member)) ||
        givers.0.iter().any(|el| !group.members.contains_key(&el.member)) {
        return Err(InternalSplitterError::InvalidName(format!("Please only specify members within the group")));
    }
    // normalize givers to contain entries for all members of the group
    let moneysplit =
        split_equal_among(total_amount - givers.1, givers.2);
    let mut wcg_index = 0;
    let mut transaction_map = HashMap::with_capacity(group.members.len());

    // positively add all the froms
    for (name, _) in &group.members {
        if let Some(giver) = givers.0.iter().find(|&target| &target.member == name)
        {
            if let Some(amount) = giver.amount {
                transaction_map.insert(name.clone(), amount);
            } else {
                transaction_map.insert(name.clone(), moneysplit[wcg_index]);
                wcg_index += 1;
            }
        } else {
            transaction_map.insert(name.clone(), 0);
        }
    }

    // subtract all tos from the balance of the transaction
    // peter started with 0, but takes 5€ of the pot, reaching a balance of -5€
    // if balance_rest is true, everything gets split onto the --to takers as well, if not, they
    // are excluded from the calculation and pay exactly as much as specified
    let moneysplit = split_equal_among(
        total_amount - recvrs.1,
        group.members.len() - if balance_rest { 0 } else { recvrs.0.len() },
    );
    let mut ms_idx = 0;
    for (name, _) in &group.members {
        if let Some(recv) = recvrs.0.iter().find(|&el| &el.member == name) {
            let x = transaction_map.get_mut(name).unwrap();
            *x -= recv.amount.unwrap();
            if balance_rest {
                *x -= moneysplit[ms_idx];
                ms_idx += 1;
            }
        } else {
            let x = transaction_map.get_mut(name).unwrap();
            *x -= moneysplit[ms_idx];
            ms_idx += 1;
        }
    }
    // if balance rest is not specified, balance between the non-specified group members
    Ok((transaction_map, givers.0, recvrs.0))
}


#[cfg(test)]
mod group_tests {
    use crate::logic::Transaction;
    use super::*;

    #[test]
    fn test_remove_member() {
        let mut group = setup_group();
        assert_eq!(group.members.len(), 4);
        let r = group.remove(vec!["Alice".to_string()], false);
        assert!(r.is_ok());
        assert_eq!(group.members.len(), 3);
        let mut group = setup_group();
        let r = group.remove(vec!["Theseus".to_string()], false);
        assert!(r.is_err());

        let mut group = setup_group();
        *group.members.get_mut("Alice").unwrap() = 100;
        let r = group.remove(vec!["Alice".to_string()], true);
        assert!(r.is_ok());
        assert_eq!(group.members.len(), 3);
    }

    #[test]
    fn test_add_member() {
        let mut group = setup_group();
        assert_eq!(group.members.len(), 4);
        let r = group.add(vec!["Egbert".to_string()]);
        assert!(r.is_ok(), "{:#?}", r.unwrap_err());
        assert_eq!(group.members.len(), 5);
        assert!(group.members.contains_key("Egbert"));
        assert_eq!(group.members["Egbert"], 0);

        let mut group = setup_group();
        assert_eq!(group.members.len(), 4);
        let r = group.add(vec!["Alice".to_string()]);
        assert!(r.is_err());
        assert_eq!(group.members.len(), 4);
    }

    #[test]
    fn test_apply_tachange() {
        let mut group =
            Group::new("testgroup".to_string(),
                       vec!["Alice".to_string(), "Bob".to_string()],
                       None).unwrap();
        let tac = TransactionChange::from(
            [("Alice".into(), -10),
                ("Bob".into(), 10)]);
        group.apply_tachange(tac);
        assert_eq!(group.members["Alice"], -10);
        assert_eq!(group.members["Bob"], 10);
    }

    #[test]
    fn test_balance_equal() {
        let mut group =
            Group::new("testgroup".to_string(),
                       vec!["Alice".to_string(), "Bob".to_string()],
                       None).unwrap();
        *(group.members.get_mut("Alice").unwrap()) = -10_00;
        *(group.members.get_mut("Bob").unwrap()) = 10_00;

        let tas = group.balance();
        assert_eq!(tas.len(), 1);
        assert_eq!(tas[0], Transaction::new("Alice", "Bob", 10_00));
    }

    #[test]
    fn test_balance_unequal() {
        let mut group =
            Group::new("testgroup".to_owned(),
                       vec!["Alice".to_string(), "Bob".to_string(),
                            "Charly".to_string(), "Django".to_string()],
                       None).unwrap();
        *(group.members.get_mut("Alice").unwrap()) = -1685;
        *(group.members.get_mut("Bob").unwrap()) = 316;
        *(group.members.get_mut("Charly").unwrap()) = 2117;
        *(group.members.get_mut("Django").unwrap()) = -748;
        let tas = group.balance();
        assert_eq!(tas[0], Transaction::new(&"Django".to_string(), &"Bob".to_string(), 316));
        assert_eq!(tas[1], Transaction::new(&"Django".to_string(), &"Charly".to_string(), 432));
        assert_eq!(tas[2], Transaction::new(&"Alice".to_string(), &"Charly".to_string(), 1685));
    }

    #[test]
    fn remove_log_test() {
        let mut group = setup_group();
        {
            let r = group.log_pay_transaction(12, "Alice".into(), "Bob".into());
            assert!(r.is_ok());
            let r = group.log_pay_transaction(13, "Alice".into(), "Bob".into());
            assert!(r.is_ok());
            assert_eq!(group.log.len(), 2);
            let r = group.remove_log(Some(1));
            assert!(r.is_ok());
            assert_eq!(group.log.len(), 1);
            match &group.log[0].command {
                LoggedCommand::Pay { amount, from, to } => {
                    assert_eq!(*amount, 12);
                    assert_eq!(from, "Alice");
                    assert_eq!(to, "Bob");
                }
                _ => unreachable!("Command is not expected Variant")
            }
            let r = group.remove_log(Some(0));
            assert!(r.is_ok());
            assert_eq!(group.log.len(), 0);
        }
        {
            let r = group.log_pay_transaction(12, "Alice".into(), "Bob".into());
            assert!(r.is_ok());
            let r = group.log_pay_transaction(13, "Alice".into(), "Bob".into());
            assert!(r.is_ok());
            let r = group.remove_log(None);
            assert!(r.is_ok());
            assert_eq!(group.log.len(), 1);
        }
    }

    #[test]
    fn test_split_equal_among() {
        // test positive values
        // test "perfect" split
        let result = split_equal_among(100, 10);
        for x in result {
            assert_eq!(x, 10);
        }
        // test "imperfect" split
        let result = split_equal_among(100, 9);
        let expected_vec = vec![12, 11, 11, 11, 11, 11, 11, 11, 11];
        for i in 0..expected_vec.len() {
            assert_eq!(result[i], expected_vec[i]);
        }
        // test negative values
        // test "perfect" split
        let result = split_equal_among(-100, 10);
        for x in result {
            assert_eq!(x, -10);
        }
        // test "imperfect" split
        let result = split_equal_among(-100, 9);
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
        let group = setup_group();

        let transaction_bins = split_into_transaction(
            120, &group, vec!["Alice".to_string()], vec![], false);
        // alle - 120/4 = -30, Alice +120 | A90, B-30,c-30, D-30
        assert!(transaction_bins.is_ok());
        let (transaction_bins, _, _) = transaction_bins.unwrap();
        assert_eq!(transaction_bins.len(), 4);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert!(transaction_bins.contains_key("Django"));
        assert_eq!(transaction_bins["Alice"], 90);
        assert_eq!(transaction_bins["Bob"], -30);
        assert_eq!(transaction_bins["Charly"], -30);
        assert_eq!(transaction_bins["Charly"], -30);
    }

    #[test]
    fn test_multiple_givers() {
        let group = setup_group();

        let transaction_bins = split_into_transaction(
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
        let group = setup_group();

        let transaction_bins = split_into_transaction(
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
        let group = setup_group();

        let transaction_bins = split_into_transaction(
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
        let group = setup_group();
        let transaction_bins = split_into_transaction(
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

    fn setup_group() -> Group {
        Group::new("testgroup".to_owned(),
                   vec!["Alice".to_string(), "Bob".to_string(),
                        "Charly".to_string(), "Django".to_string()], None).unwrap()
    }
}