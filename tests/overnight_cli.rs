use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn sc_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sc"))
}

fn write_test_config(config_dir: &Path) {
    let config = r#"[auth]
session_backend = "file"
signing_key_backend = "file"
"#;
    fs::write(config_dir.join("config.toml"), config).expect("write config");
}

fn temp_config_dir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_test_config(tmp.path());
    tmp
}

#[test]
fn help_shows_overnight_command() {
    let config_dir = temp_config_dir();

    sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show overnight savings account summary",
        ));
}

#[test]
fn overnight_help_mentions_savings_account_id() {
    let config_dir = temp_config_dir();

    sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["overnight", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--savings-account-id"));
}

#[test]
fn overnight_help_exposes_transactions_and_leaf_options() {
    let config_dir = temp_config_dir();

    sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["overnight", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("transactions"));

    sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["overnight", "transactions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--page-size"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("Backend transaction type filter"))
        .stdout(predicate::str::contains("For example"));
}

#[test]
fn overnight_json_without_session_returns_machine_envelope() {
    let config_dir = temp_config_dir();

    let assert = sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["overnight", "--json"])
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let envelope: Value = serde_json::from_str(&stdout).expect("machine json envelope");

    assert_eq!(envelope["ok"], json!(false));
    assert_eq!(envelope["command"], json!("overnight"));
    assert_eq!(envelope["error"]["code"], json!("no_session"));
    assert_eq!(
        envelope["hints"],
        json!(["Run 'sc login' first to create a session."])
    );
}

#[test]
fn capabilities_include_overnight_command() {
    let config_dir = temp_config_dir();

    let assert = sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["capabilities", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let envelope: Value = serde_json::from_str(&stdout).expect("machine json envelope");
    let commands = envelope["data"]["commands"]
        .as_array()
        .expect("capabilities commands array");

    assert!(commands.contains(&json!("overnight")));
    assert!(commands.contains(&json!("overnight.transactions")));
}

#[test]
fn overnight_transactions_json_without_session_returns_leaf_machine_envelope() {
    let config_dir = temp_config_dir();

    let assert = sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["overnight", "transactions", "--json"])
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let envelope: Value = serde_json::from_str(&stdout).expect("machine json envelope");

    assert_eq!(envelope["ok"], json!(false));
    assert_eq!(envelope["command"], json!("overnight.transactions"));
    assert_eq!(envelope["error"]["code"], json!("no_session"));
}

#[test]
fn overnight_transactions_parse_failure_uses_leaf_machine_envelope() {
    let config_dir = temp_config_dir();

    let assert = sc_command()
        .env("SC_CONFIG_DIR", config_dir.path())
        .args(["overnight", "transactions", "--page-size", "0", "--json"])
        .assert()
        .failure();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    let envelope: Value = serde_json::from_str(&stdout).expect("machine json envelope");

    assert_eq!(envelope["ok"], json!(false));
    assert_eq!(envelope["command"], json!("overnight.transactions"));
    assert_eq!(envelope["error"]["code"], json!("invalid_input"));
}
