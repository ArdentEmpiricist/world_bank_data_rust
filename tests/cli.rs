use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn cli_shows_help() {
    let mut cmd = Command::cargo_bin("wbi").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("wbi"));
}

// Live test (opt-in): cargo test --features online -- --ignored
#[cfg(feature = "online")]
#[test]
fn fetch_online_population() {
    let mut cmd = Command::cargo_bin("wbi").unwrap();
    cmd.args([
        "get",
        "--countries",
        "DEU",
        "--indicators",
        "SP.POP.TOTL",
        "--date",
        "2019:2020",
        "--stats",
        "--locale",
        "de",
    ]);
    cmd.assert().success();
}
