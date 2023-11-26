CREATE TABLE commands
(
    id   INTEGER PRIMARY KEY,
    name VARCHAR(16)
);

INSERT INTO commands (id, name)
VALUES (0, "split"),
       (1, "pay"),
       (2, "create"),
       (3, "undo");

CREATE TABLE currencies
(
    id         INTEGER PRIMARY KEY,
    sign       CHAR(1) NOT NULL,
    short_sign CHAR(3) NOT NULL
);

CREATE TABLE groups
(
    id   INTEGER PRIMARY KEY,
    name VARCHAR(64)

);

CREATE TABLE members
(
    id       INTEGER PRIMARY KEY,
    group_id INTEGER NOT NULL REFERENCES groups,
    name     TEXT    NOT NULL,
    balance  INTEGER NOT NULL
);


-- this table contains a complete log of the splitter instance you work with
CREATE TABLE undo_logging
(
    id       INTEGER PRIMARY KEY,                     -- log id
    group_id INTEGER FOREIGN KEY REFERENCES groups,   -- associated group
    command  INTEGER FOREIGN KEY REFERENCES commands, -- command
    time     VARCHAR(20) NOT NULL,                    -- timestamp of command as ISO timestamp
    amount   INTEGER                                  -- associated amount
);

-- contains details to a split/pay transaction
-- example: Fred split 12€ in the group, receiving 50% of payment(6€). Transaction ID is 5,
-- the group consists of him, and two others. charles takes 30%(3,60€) and louisa takes 20%(2,40€)
-- this would be recorded as:
-- 5, (fred), (charles), 3,60
-- 5, (fred), (louisa), 2,40
CREATE TABLE transactions
(
    log_id INTEGER FOREIGN KEY REFERENCES undo_logging,
    member INTEGER FOREIGN KEY REFERENCES members,
    amount INTEGER NOT NULL
);

