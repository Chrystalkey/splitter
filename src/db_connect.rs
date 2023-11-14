use std::collections::{HashMap};
use std::io;
use rusqlite::{params, Connection, Result, Error};
use std::path::PathBuf;
use chrono::prelude::{DateTime, Utc};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};

fn now_in_iso() -> String {
    let dt: DateTime<Utc> = std::time::SystemTime::now().into();
    format!("{}", dt.format("%+"))
}

const UNDO_LOGTABLE_NAME: &str = "undo_logging";


pub(crate) struct DatabaseConnection {
    pub(crate) path: PathBuf,
    pub(crate) connection: Connection,
}

impl DatabaseConnection {
    pub fn create(path: PathBuf, structure: &str) -> DatabaseConnection {
        let connection = Self::connect(path);
        connection.connection.execute_batch(structure)
            .expect("Could not execute Initial Database structure query");
        return connection;
    }

    pub(crate) fn connect(path: PathBuf) -> DatabaseConnection {
        let connection = Connection::open(path.as_path())
            .expect(format!("Could not open Database {:?}\n", &path).as_str());
        DatabaseConnection {
            path: path,
            connection: connection,
        }
    }
    /// query members and group_stats to get id and name of members of a group
    pub fn get_group_members(self: &Self, group_id: u32) -> Result<HashMap<String, u32>> {
        todo!()
    }
    pub fn get_group_id(self: &Self, name: &str) -> Result<i64> {
        return self.connection.query_row_and_then("SELECT id FROM group_names WHERE name = ?1", [name], |row| row.get::<_, i64>(0));
    }

    /// simply outputs the member ids that were last added to the member name table
    pub fn get_member_id_simple(self: &Self, name: &str) -> Result<i64> {
        let query = "SELECT id from members WHERE name = ?1 ORDER BY id DESC LIMIT 1";
        return Ok(self.connection.query_row_and_then(query, [name], |row| row.get(0))
            .expect(""));
    }

    /// outputs the member ids that belong to group with id `of_group`
    pub fn get_member_id(self: &Self, name: &str, of_group: i64) -> Result<i64> {
        let query =
            "SELECT m.id FROM members m, group_stats gs WHERE m.name = ?1 AND m.id = gs.member_id AND gs.group_id = ?2";
        return Ok(self.connection.query_row_and_then(query, params![name, of_group], |row| row.get(0))
            .unwrap());
    }
    pub fn get_command_id(self: &Self, cmd: &str) -> Result<i64> {
        let query = "SELECT id FROM commands WHERE name = ?1";
        return self.connection.query_row_and_then(query, [cmd], |row| row.get(0));
    }

    pub(crate) fn create_group(self: &Self, group_name: String, members: Vec<String>) -> Result<()> {
        // step one, check if the group exists, if so, return an error indicating just that
        {
            let group_exists_query = "SELECT * FROM group_names WHERE name = ?1 LIMIT 1";
            let exists = match self.connection.query_row(group_exists_query, [group_name.as_str()], |row| row.get::<_, i64>(0))
            {
                Err(_) => false,
                Ok(_) => true
            };

            if exists {
                return Err(Error::InvalidParameterName("Error: a group with this name exists and has not been deleted".to_owned()));
            }
        }

        // step two, create an entry in group names
        self.connection.execute("INSERT INTO group_names(name) VALUES (?);", [group_name.as_str()])
            .unwrap();

        let group_id = self.get_group_id(group_name.as_str()).expect("");

        // step three, create entries in members, if that member does not exist already

        let mut create_member = self.connection.prepare(
            "INSERT INTO members (name) VALUES(?1)").expect("Could not prepare Member query");

        let mut create_mem_group = self.connection.prepare(
            "INSERT INTO group_stats(group_id, member_id, amount) VALUES (?1, ?2, 0)").expect("Could not prepare group-stats query");

        let mut member_ids: Vec<i64> = Vec::with_capacity(members.len());
        for name in &members {
            create_member.execute([name.as_str()])
                .expect(format!("Error executing member name insertion query for member {name}").as_str());
            member_ids.push(self.get_member_id_simple(name.as_str())?);
            create_mem_group.execute(params![group_id, member_ids.last().unwrap()])
                .expect("Error assigning member to group");
        }

        // step five, record this command in undo_logging
        let mut ins_undo = self.connection.prepare(
            "INSERT INTO undo_logging (group_id, command, time) VALUES (?1, ?2, ?3)").unwrap();
        ins_undo.execute(params![group_id, self.get_command_id("create").unwrap(), now_in_iso().as_str()])
            .expect(format!("Could not insert group_create command into undo logging for group name {group_name}").as_str());

        return Ok(());
    }

    /// Default Group, meaning if no group is specified, this group is selected.
    /// if no group is found (because no create action has been performed) this function
    /// returns error
    pub(crate) fn default_group(self: &Self) -> Result<(u32, String)> {
        let query = "SELECT gr.id, gr.name FROM group_names gr, undo_logging ul WHERE ul.group = gr.id ORDER BY ul.id DESC LIMIT 1;";
        match self.connection
            .query_row(query, [],
                       |row|
                           Ok((row.get::<_, i64>(0).unwrap(), row.get::<_, String>(1).unwrap())),
            ) {
            Ok(_) => Ok((0, "".to_owned())),
            _ => { Err(Error::InvalidParameterName("No default group was found, quite possibly the thing is empty".to_owned())) }
        }
    }
}
