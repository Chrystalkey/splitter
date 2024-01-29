#[cfg(test)]
mod integration {
    use std::fs;
    use std::io::{BufWriter, Write};
    #[cfg(windows)]
    use std::os::windows::fs::MetadataExt;

    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    const DB_NAME: &str = "test.db";

    fn cleanup(db_name: &str) {
        if PathBuf::from(db_name).exists() {
            fs::remove_file(db_name).unwrap();
        }
    }

    fn create_group(db_name: &str) {
        let exit_status = Command::new("cargo")
            .args(&["run", "--", "-d", db_name, "create", "testgroup", "-a", "alice", "-a", "bob", "-a", "charly", "-a", "django"])
            .spawn()
            .unwrap()
            .wait();
        assert!(exit_status.is_ok() && exit_status.unwrap().success());
    }

    fn insert_split() {}

    // choose a unique db name for different test functions due to multithread madness
    #[test]
    fn test_create() {
        // non-existing group, should not fail
        let db_filename = format!("test_crt_{}", DB_NAME);
        cleanup(db_filename.as_str());
        let exit_status = Command::new("cargo")
            .args(&["run", "--", "-d", db_filename.as_str(), "create", "testgroup", "-a", "alice", "-a", "bob"])
            .spawn()
            .unwrap()
            .wait();
        assert!(exit_status.is_ok());
        assert!(exit_status.as_ref().unwrap().success(), "Instead: {:?}", exit_status.unwrap().code());
        // existing group, should fail
        let exit_status = Command::new("cargo")
            .args(&["run", "--", "-d", db_filename.as_str(), "create", "testgroup", "-a", "alice", "-a", "bob"])
            .spawn()
            .unwrap()
            .wait();
        assert!(exit_status.is_ok());
        assert!(!exit_status.unwrap().success());
        cleanup(db_filename.as_str());
    }

    #[test]
    fn test_delete_group() {
        // delete existing group, should not fail
        let db_filename = format!("test_del_{}", DB_NAME);
        cleanup(db_filename.as_str());
        {
            create_group(db_filename.as_str());
            let fsize = fs::metadata(db_filename.as_str()).expect("Should have given file size");
            let mut child = Command::new("cargo")
                .args(&["run", "--", "-d", db_filename.as_str(), "delete-group", "testgroup"])
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
            assert!(fsize.file_size() >
                fs::metadata(db_filename.as_str()).expect("Should have given file size").file_size());
            cleanup(db_filename.as_str());
        }

        // delete non-existing group, should give an error and not modify the database
        {
            create_group(db_filename.as_str());
            let fsize = fs::metadata(db_filename.as_str()).expect("Should have given file size");
            let mut child = Command::new("cargo")
                .args(&["run", "--", "-d", db_filename.as_str(), "delete-group", "anderegruppe"])
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();
            let exit_status = child.wait();
            assert!(exit_status.is_ok());
            assert!(!exit_status.unwrap().success());
            assert_eq!(fsize.file_size(),
                       fs::metadata(db_filename.as_str()).expect("Should have given file size").file_size());
            cleanup(db_filename.as_str());
        }

        // delete existing group and abort, should give no error and have no effects
        {
            create_group(db_filename.as_str());
            let fsize = fs::metadata(db_filename.as_str()).expect("Should have given file size");
            let mut child = Command::new("cargo")
                .args(&["run", "--", "-d", db_filename.as_str(), "delete-group", "anderegruppe"])
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();
            {
                let stdin_of_child = child.stdin.as_mut().unwrap();
                let mut writer = BufWriter::new(stdin_of_child);
                writer.write("n".as_bytes()).unwrap();
            }
            let exit_status = child.wait();
            assert!(exit_status.is_ok());
            assert!(!exit_status.unwrap().success());
            assert_eq!(fsize.file_size(),
                       fs::metadata(db_filename.as_str()).expect("Should have given file size").file_size());
            cleanup(db_filename.as_str());
        }

        // delete existing group and confirm, should not fail and erase the group from the database
        {
            create_group(db_filename.as_str());
            let fsize = fs::metadata(db_filename.as_str())
                .expect("Should have given file size")
                .file_size();
            let mut child = Command::new("cargo")
                .args(&["run", "--", "-d", db_filename.as_str(), "delete-group", "testgroup"])
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
            assert!(fsize >
                fs::metadata(db_filename.as_str()).expect("Should have given file size").file_size());
            cleanup(db_filename.as_str());
        }
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