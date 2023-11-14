# Splitter

A command line money splitting application

## Usage

The program should maintain a persistent storage on the pc, that can be specified, or defaulted.
The Default Storage location is in  ~/.splitter

The Commands that are allowed are as follows:

### Split

```split```  
```[--group {name}]``` The Group This Split is meant to belong to. Optional because of the default-group funcitonality

```[--from {member name}:number[%]]+``` Repeatable Argument, parsed in the order in which it was passed.
If a % is specified at the end of amount, it is interpreted as percentage, if not as an absolute amount.
If the sum is more than 100% of the amount payed, the action fails.
--from fred 200% -> is discarded  
--from a 10% --from b 20% --from c 80% -> is discarded  
at least one instance of a from argument must be present  

```[--to {member name}:number[%]]*``` Repeatable Argument, telling the program who "takes" how much of the "pot".
("To whom go 3,6")
For example, A Pizza restaurant bill costs 22,5€ and is split between fred, george and Jenny.
Fred pays for himself, george and Jenny split whats left among them.
This would look like: [...] --to fred 3,6
If all amounts are specified and some of the pot is leftover, a warning is displayed and the rest is split evenly
among all members of the group.

```[--balance-rest]``` if this switch is specified, the non-specified --to amounts are split among all members of the
group
evenly, doubling down on the amounts specified via --to.
An example: Fred wants to pay just for is wine, which is 2,20€, the rest of the matter is split among all of the
group.:  
```splitter split 20 --to fred 2,20 --balance-rest```

There are two positional arguments, the first one denoting the amount to be split, the second one is the payment's name.
A sample call using only mandatory options looks like this:  
```splitter 5 Rewe```

A sample call using all options looks like this:
```splitter 20 Rewe --from fred:3,5 --from jenny:30% --to fred:3,5 --group spezi```

### Pay

```pay```

group is defined as above

```--from {member name}``` a name of a member within the specified group

```--to {member name}``` a name of a member within the specified group

Used like:
```splitter pay {amount} [--group {name}] --from {member name} --to {member name}```

### Undo

splitter undo -> undo the last splitting action

### Create

splitter create {group name} [--add {name} [{name} ...]]

Deletes the group specified as third argument

### List

splitter list [{group name}] -> lists the group name or if none is specified all groups and their expenses  
Lists a few(all) transactions from a group numbered in a way that is deletable

### Stat

splitter stat [[--group] {group name}] -> shows the stats of a given group or if none is specified all groups

print out statistics of the group (who owes whom how much)

### Delete Entry

splitter delete-entry [[--group] {group name}] {entry number}

### Delete Group

```splitter delete-group {group name} [--group {group name}]```

### Balance

splitter balance [[--group] {group name}] -> shows what has to be paid to whom and sets expenses such that everything is
payed up afterwards