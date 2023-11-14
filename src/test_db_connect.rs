use crate::db_connect::DatabaseConnection;

struct DebugDB(DatabaseConnection);

impl Drop for DebugDB {
    fn drop(&mut self) {
        std::fs::remove_file(Self.0.path).expect("Could not remove file");
    }
}

fn build_sample_db() -> DebugDB {
    let structure_string = std::fs::read_to_string("src/db_structure.sql")
        .expect("Error: could not read db structure file");
    let dbcon = DebugDB { 0: DatabaseConnection::create("testdb.sqlite".into(), structure_string.as_str()) };
    dbcon.0.create_group("group".to_owned(),
                         vec!["member1".to_owned(), "member2".to_owned(), "member3".to_owned()])
        .expect("Could not create group. failed.");
    return dbcon;
}

#[test]
fn test_create() {
    let structure_string = std::fs::read_to_string("src/db_structure.sql")
        .expect("Error: could not read db structure file");
    let dbcon = DebugDB { 0: DatabaseConnection::create("testdb.sqlite".into(), structure_string.as_str()) };
    dbcon.0.create_group("group".to_owned(),
                         vec!["member1".to_owned(), "member2".to_owned(), "member3".to_owned()])
        .expect("Could not create group. failed.");
    let gid = dbcon.0.get_group_id("group").expect("Could not get group id");
    assert_eq!(gid, 1);
    assert_eq!(dbcon.0.get_member_id("member1", gid)
                   .expect("Could not get member Id of member1"), 1);
}

#[test]
fn test_getters() {}

#[test]
fn test_creategroup() {}