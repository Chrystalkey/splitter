#[cfg(test)]
mod integration {
    use std::fs;
    use std::io::{BufWriter, Write};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    const DB_NAME: &str = "test.db";

    fn cleanup() {
        if PathBuf::from(DB_NAME).exists() {
            fs::remove_file(DB_NAME).unwrap();
        }
    }

    fn create_group() {
        let exit_status = Command::new("cargo")
            .args(&["run", "--", "-d", DB_NAME, "create", "testgroup", "-a", "alice", "-a", "bob", "-a", "charly", "-a", "django"])
            .spawn()
            .unwrap()
            .wait();
    }

    fn insert_split() {}

    // choose a unique db name for different test functions due to multithread madness
    #[test]
    fn test_create() {
        // non-existing group, should not fail
        cleanup();
        let exit_status = Command::new("cargo")
            .args(&["run", "--", "-d", DB_NAME, "create", "testgroup", "-a", "alice", "-a", "bob"])
            .spawn()
            .unwrap()
            .wait();
        assert!(exit_status.is_ok());
        assert!(exit_status.unwrap().success());
        // existing group, should fail
        let exit_status = Command::new("cargo")
            .args(&["run", "--", "-d", DB_NAME, "create", "testgroup", "-a", "alice", "-a", "bob"])
            .spawn()
            .unwrap()
            .wait();
        assert!(exit_status.is_ok());
        assert!(!exit_status.unwrap().success());
        cleanup();
    }

    #[test]
    fn test_delete_group() {
        // delete existing group, should not fail
        create_group();
        let mut child = Command::new("cargo")
            .args(&["run", "--", "-d", DB_NAME, "delete-group", "testgroup"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let stdin_of_child = child.stdin.as_mut().unwrap();
            let mut writer = BufWriter::new(stdin_of_child);
            writer.write("y".as_bytes()).unwrap();
        }
        let exit_status = child.wait();
        assert!(exit_status.is_ok());
        assert!(exit_status.unwrap().success());
        cleanup();

        // delete non-existing group, should give an error and not modify the database
        create_group();
        let mut child = Command::new("cargo")
            .args(&["run", "--", "-d", DB_NAME, "delete-group", "anderegruppe"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let exit_status = child.wait();
        assert!(exit_status.is_ok());
        assert!(!exit_status.unwrap().success());
        cleanup();

        // delete existing group and abort, should give no error and have no effects

        // delete existing group and confirm, should not fail and erase the group from the database
    }

    #[test]
    fn test_split() {
        // split on non-existing group, should fail

        // split from non-existing member, should fail
        // split to non-existing member, should fail
        // split from an existing member
        // split to existing member
        // split from and to existing members

        // split amount < explicitly specified amounts in from and to
        // split amount > explicitly specified amounts in from and to with no wildcard

        // split amount with wildcard on existing group with existing members. should work.
    }

    #[test]
    fn test_balance() {
        // balance on non-existing group
        // balance on existing group
        // balance on 0-group, should not change anything
        // balance on non-0-group, should change
        // balance on non-0-group and abort, should not change anything
    }

    #[test]
    fn test_add() {
        // add to non-existent group
        // add to existing group member with the same name

        // add to existing group member with different name
    }

    #[test]
    fn test_remove() {
        // remove from non-existing group
        // remove from existing group non-existent member
        // remove from existing group existing member with non-0-balance

        // remove from existing group existing member with non-0-balance + force
        // remove from existing group existing member with 0-balance
    }
}