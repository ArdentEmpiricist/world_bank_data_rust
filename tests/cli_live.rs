//! Live CLI test. Run with: `cargo test --features online --test cli_live -- --nocapture`
#![cfg(feature = "online")]

use std::process::Command;

#[test]
fn run_cli_get_and_plot() {
    // Cargo sets this to the path of the compiled binary named as in Cargo.toml [[bin]].
    let exe = env!("CARGO_BIN_EXE_wbi_rs");
    let tmp = std::env::temp_dir().join("wbd_cli_plot.svg");
    let status = Command::new(exe)
        .args([
            "get",
            "--countries",
            "DEU",
            "--indicators",
            "SP.POP.TOTL",
            "--date",
            "2019:2020",
            "--plot",
            tmp.to_str().unwrap(),
            "--legend",
            "right",
        ])
        .status()
        .expect("spawn cli");
    assert!(status.success());
    assert!(std::fs::metadata(&tmp).is_ok());
    std::fs::remove_file(&tmp).ok();
}
