use assert_cmd::prelude::*;
use std::process::Command;
use predicates::prelude::*;

#[test]
fn cli_shows_help() {
    let mut cmd = Command::cargo_bin("world_bank_data_rust").unwrap();
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains("world_bank_data_rust"));
}

// Live test (opt-in): cargo test --features online -- --ignored
#[cfg(feature="online")]
#[ignore]
#[test]
fn fetch_online_population() {
    let mut cmd = Command::cargo_bin("world_bank_data_rust").unwrap();
    cmd.args([
        "get", "--countries", "DEU", "--indicators", "SP.POP.TOTL",
        "--date", "2019:2020", "--stats", "--locale", "de"
    ]);
    cmd.assert().success();
}