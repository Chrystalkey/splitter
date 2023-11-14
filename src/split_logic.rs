use std::collections::HashMap;
use std::path::PathBuf;
use crate::config::SubCommand;

type MemberID = usize;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq)]
enum Money {
    Euro(u32),
    USD(u32),
}

impl Into<f32> for Money {
    fn into(self) -> f32 {
        match self {
            Self::Euro(cents) |
            Self::USD(cents) => { cents as f32 / 100. }
            _ => { panic!("Could not convert your currency"); }
        }
    }
}

impl ToString for Money {
    fn to_string(&self) -> String {
        match self {
            Money::Euro(amount) => format!("{} €", amount),
            Money::USD(amount) => format!("{} $", amount),
            _ => { unreachable!() }
        }
    }
}

struct TransactionTodo {
    amount: Money,
    from: MemberID,
    to: MemberID,
}

type TransactionChange = HashMap<String, Money>;

struct Member {
    id: MemberID,
    name: String,
}

struct LogEntry {
    command: SubCommand,
    change: Option<TransactionChange>,
}

struct Group {
    name: String,
    members: Vec<Member>,
    transaction_todos: Vec<TransactionTodo>,
    log: Vec<LogEntry>,
}

impl Group {
    fn new(name: String, members: Vec<String>) -> Self {
        let membrs = {
            let mut vec = Vec::with_capacity(members.len());
            let mut index = 0;
            for m in members {
                vec.push(Member { id: index, name: m });
                index += 1;
            }
            vec
        };
        Self {
            name,
            members: membrs,
            transaction_todos: vec![],
            log: vec![],
        }
    }
}

struct SplitterState {
    groups: Vec<Group>,
}

impl SplitterState {
    fn parse(source: PathBuf) -> Self {
        todo!();
    }
}

/// helper struct containing money and a name. Can be used as a "from" or as a "to"
struct FromTo {
    member: String,
    amount: Money,
}

impl FromTo {
    fn parse(input: String, total_money: &Money) -> Self {
        let in_split: Vec<_> = input.trim_end_matches("%").split(":").collect();
        if in_split.len() != 2 {
            panic!("Error: Please use the following syntax to specify Amounts: --from/--to <name>:<amount>[%]")
        }

        let amount = if input.ends_with("%") {
            let percent: f32 = in_split[1].parse()
                .expect(format!("Please specify a valid number as amount for argument {}", input).as_str());
            let amount = if let Money::Euro(cur_amount) = total_money {
                (percent * *cur_amount as f32) as u32 // here is a silent *e-2 * e+2 for dividing the percentage
            } else {
                todo!();
            };
            amount
        } else {
            let amount: f32 = in_split[1].parse::<f32>()
                .expect(format!("Error: Please specify a valid number as amount for argument {}", input).as_str())
                * 100.;
            amount as u32
        };
        return Self {
            member: in_split[0].to_owned(),
            amount: Money::Euro(amount),
        };
    }
}

struct Logic {
    state: SplitterState,

    current_group: Option<usize>,
    currency: String,
}

impl Logic {
    const NAME_REGEX: &'static str = r"[a-zA-Z0-9][a-zA-Z0-9_\-()]*";
    fn new(source: PathBuf) -> Self {
        let state = SplitterState::parse(source);
        let current_group = if state.groups.is_empty() { None } else { Some(0) };
        return Self {
            state,
            current_group,
            currency: "€".to_owned(),
        };
    }

    fn create_group(self: &mut Self, name: String, members: Vec<String>) -> &Group {
        self.state.groups.push(Group::new(name, members));
        return self.state.groups.last().unwrap();
    }

    fn stat(self: &Self, group_name: Option<String>) {
        let group = self.get_group(group_name);
        println!("Group Statistics for group {}:", group.name);
        println!("Members:");
        for mem in &group.members {
            println!("{}: {}", mem.id, mem.name);
        }
        println!("\n\nOutstanding Transactions:");
        for ta in &group.transaction_todos {
            println!("From {} to {}: {} {}",
                     group.members[ta.from].name, group.members[ta.to].name,
                     ta.amount.to_string(), self.currency);
        }
    }

    /// get a reference to the group or panic
    fn get_group(self: &Self, group_name: Option<String>) -> &Group {
        let group = match group_name {
            None => (self.state.groups.get(self.current_group.expect("Error: No group was found"))).unwrap(),
            Some(name) => self.state.groups.iter().find(|&gn| gn.name == name)
                .expect("Error: Could not find a group with this name")
        };
        return group;
    }

    fn parse_froms_and_tos(from: Vec<String>, to: Vec<String>, amount: Money)
                           -> (Vec<FromTo>, Vec<FromTo>) {
        if from.len() < 1 {
            panic!("Error: Needs at least one Argument \"--from\"");
        }
        let mut froms_parsed = Vec::with_capacity(from.len());
        let mut given_amount = Money::Euro(0);
        for giver in from {
            froms_parsed.push(FromTo::parse(giver, &amount));
            given_amount.into() += froms_parsed.last().unwrap().amount.0;
        }
        if given_amount > amount {
            panic!("Error: The amounts specified with '--from' sum up to more than the total amount: {} vs {}",
                   given_amount.0, amount.0);
        }
        let mut tos_parsed = Vec::with_capacity(to.len());
        let mut taken_amount = Money::Euro(0);
        for recv in to {
            tos_parsed.push(FromTo::parse(recv, &amount));
            taken_amount.0 += tos_parsed.last().unwrap().amount.0;
        }
        if taken_amount > amount {
            panic!("Error: The amounts specified with '--to' sum up to more than the total amount: {} vs {}",
                   taken_amount.0, amount.0);
        }
        return (froms_parsed, tos_parsed);
    }
    fn split(self: &Self, amount: Money, group_name: Option<String>,
             from: Vec<String>, to: Vec<String>, name: String) {
        let transaction = self.split_into_transaction(amount, group_name, from, to);
    }
    fn split_into_transaction(self: &Self, amount: Money, group_name: Option<String>,
                              from: Vec<String>, to: Vec<String>) -> TransactionChange {
        let (fr_parsed, to_parsed) = Self::parse_froms_and_tos(from, to, amount);
        let group = self.get_group(group_name);
        let mut bins = HashMap::<String, f32>::with_capacity(group.members.len());

        for member in &group.members {
            bins.insert(member.name.clone(), 0.);
        }
        let mut to_sum = 0;
        for to in to_parsed {
            bins.get_mut(to.member.as_str())
                .expect(format!("Error: No member '{}' could be found in group '{}'", to.member, group.name).as_str())
                -= to.amount;
            to_sum += to.amount;
        }
        // then positively add all the froms
        for from in &fr_parsed {
            bins.get_mut(from.member.as_str())
                .expect(format!("Error: No member '{}' could be found in group '{}'", to.member, group.name).as_str())
                += from.amount;
        }
        let rest_amount = amount - to_sum;
        let rest_amount_pp = rest_amount / (group.members.len() - to_parsed.len());
        for member in &group.members {
            if !to_parsed.iter().find(|&el| el.member == member).is_some() {
                bins.get_mut(member.name.as_str())
                    .expect(format!("Error: No member '{}' could be found in group '{}'", to.member, group.name).as_str())
                    -= rest_amount_pp;
            }
        }
        // handle entries that are a fraction of a cent
        bins.drain().map(|(k, v)| (k, Money::Euro(v as u32))).collect()
    }

    fn run(self: &Self, command: SubCommand) -> Result<(), &str> {
        match command {
            SubCommand::Create { name, members } => todo!(),
            SubCommand::Undo => { todo!() }
            SubCommand::DeleteGroup { group } => todo!(),
            SubCommand::DeleteEntry { group, entry_number } => todo!(),
            SubCommand::List { group } => todo!(),
            SubCommand::Stat { group } => todo!(),
            SubCommand::Pay { amount, group, from, to } => todo!(),
            SubCommand::Split {
                amount, group, from, to, name, balance_rest
            } => { todo!(); }
            SubCommand::Balance { group } => todo!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::split_logic::{Group, Logic, Member, Money, SplitterState};

    #[test]
    fn test_simple_split_one_from() {
        let group = Group {
            name: "testgroup".to_owned(),
            members: vec![
                Member {
                    id: 0,
                    name: "Alice".to_owned(),
                }, Member {
                    id: 0,
                    name: "Bob".to_owned(),
                }, Member {
                    id: 0,
                    name: "Charly".to_owned(),
                },
            ],
            transaction_todos: vec![],
            log: vec![],
        };
        let splitter = Logic {
            state: SplitterState {
                groups: vec![group]
            },
            current_group: Some(0),
            currency: "EUR".to_string(),
        };
        let transaction_bins = splitter.split_into_transaction(
            Money::Euro(120), Some("testgroup".to_string()), vec!["Alice".to_string()], vec![]);
        // alle - 120/3 = -40, Alice +120 | A80, B-40,c-40
        assert_eq!(transaction_bins.len(), 3);
        assert!(transaction_bins.contains_key("Alice"));
        assert!(transaction_bins.contains_key("Bob"));
        assert!(transaction_bins.contains_key("Charly"));
        assert_eq!(transaction_bins["Alice"], 80);
        assert_eq!(transaction_bins["Bob"], -40);
        assert_eq!(transaction_bins["Charly"], -40);
    }
}