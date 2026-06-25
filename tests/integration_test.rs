use assert_cmd::Command;
use predicates::boolean::PredicateBooleanExt;
use predicates::str::contains;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_directory() -> TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let dir_path = temp_dir.path();

    // Create a subdirectory
    let sub_dir_path = dir_path.join("subdir");
    std::fs::create_dir(&sub_dir_path).expect("Failed to create subdirectory");

    // Create some files in the temporary directory
    std::fs::write(dir_path.join("file1.txt"), "This is file 1. hello")
        .expect("Failed to create file1.txt");
    std::fs::write(dir_path.join("file2.txt"), "This is file 2. hello world")
        .expect("Failed to create file2.txt");
    std::fs::write(sub_dir_path.join("file3.txt"), "foo bar baz")
        .expect("Failed to create file3.txt");

    temp_dir
}

#[test]
fn prints_help() {
    Command::cargo_bin("file-finder")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("USAGE: file-finder"));
}

#[test]
fn can_find_files_using_and() {
    let temp_dir = setup_directory();
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("using-and.yaml");

    Command::cargo_bin("file-finder")
        .unwrap()
        .arg(config_path)
        .arg("--directory")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(contains("file2.txt"))
        .stdout(contains("file1.txt").not())
        .stdout(contains("subdir/file3.txt").not());
}
