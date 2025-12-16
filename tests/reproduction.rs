use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn node_modules_is_included_by_default_if_not_gitignored() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = assert_fs::TempDir::new()?;

    // Create a file inside node_modules
    let node_modules = temp.child("node_modules");
    node_modules.create_dir_all()?;
    let pkg_json = node_modules.child("package.json");
    pkg_json.write_str("{\"name\": \"test\"}")?;

    let mut cmd = cargo_bin_cmd!("pc");
    cmd.current_dir(&temp)
        .arg("-t")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("node_modules/package.json"));

    Ok(())
}
