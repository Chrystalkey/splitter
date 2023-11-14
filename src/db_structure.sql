CREATE TABLE group_names
(
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE members
(
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE group_stats
(
    group_id  INTEGER NOT NULL REFERENCES group_names,
    member_id INTEGER NOT NULL REFERENCES members,
    amount    REAL    NOT NULL,
    PRIMARY KEY (group_id, member_id)
);

CREATE TABLE commands
(
    id   INTEGER PRIMARY KEY,
    name VARCHAR(64)
);

INSERT INTO commands (id, name)
VALUES (0, "split"),
       (1, "pay"),
       (2, "create"),
       (3, "delete-group"),
       (4, "delete-entry"),
       (5, "list"),
       (6, "stat"),
       (7, "balance");

-- this table contains a complete log of the splitter instance you work with
CREATE TABLE undo_logging
(
    id         INTEGER PRIMARY KEY,                     -- log id
    group_id   INTEGER REFERENCES group_names NOT NULL, -- associated group
    command    INTEGER REFERENCES commands    NOT NULL, -- command
    time       VARCHAR(20)                    NOT NULL, -- timestamp of command as ISO timestamp
    amount     REAL,                                    -- associated amount
    parameters VARCHAR(256)                             -- other parameters, such as amount distribution, entry number etc.
);

-- contains details to a split/pay transaction
-- example: Fred split 12€ in the group, receiving 50% of payment(6€). Transaction ID is 5,
-- the group consists of him, and two others. charles takes 30%(3,60€) and louisa takes 20%(2,40€)
-- this would be recorded as:
-- 5, (fred), (charles), 3,60
-- 5, (fred), (louisa), 2,40

CREATE TABLE transactions
(
    id             INTEGER NOT NULL REFERENCES undo_logging, -- the transaction number as recorded in undo_logging
    from_member_id INTEGER NOT NULL REFERENCES members,      -- the "from" part of the partial transaction
    to_member_id   INTEGER REFERENCES members,               -- the "to" part of the partial transaction
    amount_from_to REAL    NOT NULL,                         -- the amount payed from "from" to "to"
    PRIMARY KEY (id, from_member_id, to_member_id)
);
