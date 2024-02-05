# Splitter

A command line money splitting application

## Usage

The program should maintain a persistent storage on the pc, that can be specified, or defaulted.
The Default Storage location is in  ~/.config/splitter/default.db

The Commands that are allowed are as follows:

### Split

`split`  
`[--group {name}]` The Group This Split is meant to belong to. Optional because of the default-group funcitonality

`[--from {member name}:number[%]]+` Repeatable Argument, parsed in the order in which it was passed.
If a % is specified at the end of amount, it is interpreted as percentage, if not as an absolute amount.
If the sum is more than 100% of the amount payed, the action fails.
--from fred 200% -> is discarded  
--from a 10% --from b 20% --from c 80% -> is discarded  
at least one instance of a from argument must be present

`[--to {member name}:number[%]]*` Repeatable Argument, telling the program who "takes" how much of the "pot".
("To whom go 3,6?")
For example, A Pizza restaurant bill costs 22,5€ and is split between fred, george and Jenny.
Fred pays for himself, george and Jenny split whats left among them.
This would look like: [...] --to fred 3,6
If all amounts are specified and some of the pot is leftover, a warning is displayed and the rest is split evenly
among all members of the group.

`[--balance-rest]` if this switch is specified, the non-specified --to amounts are split among all members of the
group
evenly, doubling down on the amounts specified via --to.
An example: Fred wants to pay just for is wine, which is 2,20€, the rest of the matter is split among all of the
group.:  
`splitter split 20 --to fred 2,20 --balance-rest`

There are two positional arguments, the first one denoting the amount to be split, the second one is the payment's name.
A sample call using only mandatory options looks like this:  
`splitter 5 Rewe`

A sample call using all options looks like this:
`splitter 20 Rewe --from fred:3,5 --from jenny:30% --to fred:3,5 --group spezi`

### Pay

`pay`

group is defined as above

`--from {member name}` a name of a member within the specified group

`--to {member name}` a name of a member within the specified group

Used like:
`splitter pay {amount} [--group {name}] --from {member name} --to {member name}`

### Undo

`splitter undo [{group name}] [{index}]` -> undo a splitting action if group name is not specified, the currently
selected group is used if index is not specified, the last splitting action that was not an undo from the group
specified is undone

### Create

`splitter create {group name} [--add {name}]+`

Creates the group specified, and adds the members as specified

### List

`splitter list [{group name}] [--all]` -> lists the group name or if `--all` is specified all groups and their
expenses  
Lists a few(all) transactions from a group numbered in a way that is deletable

### Stat

`splitter stat [[--group] {group name}] [--all]` -> shows the stats of a given group or if none is specified all groups

print out statistics of the group (who owes whom how much)

### Delete Group

`splitter delete-group {group name}`
Deletes the group specified

### Balance

`splitter balance [[--group] {group name}]` ->
shows what has to be paid to whom and sets expenses such that everything
is payed up afterwards, minimizing tedious transactions and amount of transactions

### add
`splitter add [--group {group name}] {member}+`
adds members to a group, silently deduplicating members with the same name.
not undoable.

### remove
`splitter remove [--group {group name}] [--force] {member}+`
removes members from a group.
Note that it is required for members to have a balance of 0.
Fails with an error message if that is not the case and `--force` is not set.
If more than one member is specified, `--force` applies to all of them.

## Project State

- [x] commands
    - [x] create
    - [x] split
    - [x] pay
    - [x] stat
    - [x] delete-group
    - [x] balance
    - [x] undo
- [ ] other features
    - [ ] interactive prompt if called with no arguments
    - [x] adding / removing members of a group after creation
    - [ ] test cases that work on the executable directly

## About splitting

## About balance

Principles that should hold for balance operations:

- no one who gets money pays money
- matching amounts should be settled with each other
- larger settlements are preferable to smaller settlements
- round settlements are prefereable

This leads to

1. settle matching amounts first
2. settle up from small debts, meaning:
    1. smallest debt pays smallest debtor, second smallest debtor until empty
    2. second smallest pays leftover smallest debtor etc

In practice this means:
sort creditors by outstanding amount, asc
sort debtors by payable amount, desc (abs asc)

## About undo
undoable commands are
- split
- pay
- create group

listings should include numbered log entries
undo without arguments should undo the last command

- fetch the last group
- fetch the last log entry
- commence undoing

undo with a number should undo the list with that number of the last group

- fetch the last group
- fetch the log entry with that number
- commence undoing

undo with a group and number argument should do just what it says.
