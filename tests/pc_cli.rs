use std::error::Error;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn prints_python_files_with_headers() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    let src_dir = temp.child("src");
    src_dir.create_dir_all()?;

    let main_py = src_dir.child("main.py");
    main_py.write_str("print('hello')\n")?;

    let ignored_txt = src_dir.child("ignored.txt");
    ignored_txt.write_str("this should not appear\n")?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("py")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "========== FILE: src/main.py ==========",
        ))
        .stdout(predicate::str::contains("print('hello')"))
        .stdout(predicate::str::contains("ignored.txt").not());

    Ok(())
}

#[test]
fn respects_gitignore_by_default() -> TestResult {
    let temp = assert_fs::TempDir::new()?;

    temp.child(".gitignore").write_str("ignored.py\n")?;

    let included = temp.child("included.py");
    included.write_str("print('included')\n")?;

    let ignored = temp.child("ignored.py");
    ignored.write_str("print('ignored')\n")?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("py")
        .assert()
        .success()
        .stdout(predicate::str::contains("included.py"))
        .stdout(predicate::str::contains("ignored.py").not());

    Ok(())
}

#[test]
fn exclude_glob_skips_matching_paths() -> TestResult {
    let temp = assert_fs::TempDir::new()?;

    let src = temp.child("src");
    let tests = temp.child("tests");
    src.create_dir_all()?;
    tests.create_dir_all()?;

    src.child("main.py").write_str("print('main')\n")?;
    tests
        .child("test_example.py")
        .write_str("print('test')\n")?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("py")
        .arg("--exclude")
        .arg("tests/**")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.py"))
        .stdout(predicate::str::contains("tests/test_example.py").not());

    Ok(())
}

#[test]
fn strip_comments_flag_removes_full_line_comments_only() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    let f = temp.child("sample.py");
    f.write_str(
        r#"# full-line comment
print("code")  # inline comment

    # indented full-line comment
"#,
    )?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("py")
        .arg("--strip-comments")
        .assert()
        .success()
        .stdout(predicate::str::contains("FILE: sample.py"))
        .stdout(predicate::str::contains("# full-line comment").not())
        .stdout(predicate::str::contains("indented full-line comment").not())
        .stdout(predicate::str::contains("inline comment"));

    Ok(())
}

#[test]
fn max_bytes_skips_large_files_and_logs_to_stderr() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    let f = temp.child("big.py");

    // Create a >50-byte file
    let content = "print('x')\n".repeat(10);
    f.write_str(&content)?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("py")
        .arg("--max-bytes")
        .arg("50")
        .assert()
        .success()
        .stdout(predicate::str::contains("big.py").not())
        .stderr(predicate::str::contains("Skipping big.py"));

    Ok(())
}

#[test]
fn path_after_type_is_not_consumed_as_another_type() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    let runner = temp.child("runner");
    runner.create_dir_all()?;

    let repo = temp.child("repo");
    repo.create_dir_all()?;
    repo.child("src").create_dir_all()?;
    repo.child("src/main.rs").write_str("fn main() {}\n")?;

    // Run from a different directory, and pass repo path explicitly.
    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&runner)
        .arg("-t")
        .arg("rs")
        .arg(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("FILE: src/main.rs"))
        .stdout(predicate::str::contains("fn main() {}"));

    Ok(())
}

#[test]
fn json_output_is_valid() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    let src_dir = temp.child("src");
    src_dir.create_dir_all()?;

    let main_py = src_dir.child("main.py");
    main_py.write_str("print('hello')\n")?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("py")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "[\n{\"path\":\"src/main.py\",\"file_name\":\"main.py\",\"content\":\"print('hello')\\n\"}\n]",
        ));

    Ok(())
}

#[test]
fn nested_gitignore_is_respected() -> TestResult {
    let temp = assert_fs::TempDir::new()?;
    let root_ignore = temp.child(".gitignore");
    root_ignore.write_str("root_ignored.txt\n")?;
    temp.child("root_ignored.txt").write_str("ignore me")?;
    temp.child("root_included.txt").write_str("include me")?;

    let nested = temp.child("nested");
    nested.create_dir_all()?;
    nested
        .child(".gitignore")
        .write_str("nested_ignored.txt\n")?;
    nested
        .child("nested_ignored.txt")
        .write_str("ignore me too")?;
    nested
        .child("nested_included.txt")
        .write_str("include me too")?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("txt")
        .assert()
        .success()
        .stdout(predicate::str::contains("root_included.txt"))
        .stdout(predicate::str::contains("nested/nested_included.txt"))
        .stdout(predicate::str::contains("root_ignored.txt").not())
        .stdout(predicate::str::contains("nested/nested_ignored.txt").not());

    Ok(())
}
