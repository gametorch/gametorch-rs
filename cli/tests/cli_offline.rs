//! Offline CLI smoke tests: help, version and argument validation.
//!
//! These never touch the network.

mod common;

use common::{run_err, run_ok};

#[test]
fn help_lists_commands() {
    let out = run_ok(&["--help"]);
    for command in [
        "catalog",
        "project",
        "sprite",
        "sound",
        "animation",
        "saved",
        "label",
        "art-style",
        "usage",
        "key",
        "health",
    ] {
        assert!(out.contains(command), "help output missing {command:?}");
    }
}

#[test]
fn version_prints() {
    let out = run_ok(&["--version"]);
    assert!(
        out.contains("gametorch"),
        "unexpected version output: {out}"
    );
}

#[test]
fn unknown_subcommand_fails() {
    let err = run_err(&["definitely-not-a-command"]);
    assert!(!err.is_empty());
}

#[test]
fn subcommand_help_works() {
    for args in [
        vec!["sprite", "--help"],
        vec!["sprite", "generate", "--help"],
        vec!["key", "create", "--help"],
        vec!["animation", "export", "--help"],
        vec!["label", "associate", "--help"],
    ] {
        let out = run_ok(&args);
        assert!(out.contains("Usage"), "no usage in {args:?}: {out}");
    }
}

#[test]
fn missing_required_argument_fails() {
    let err = run_err(&["project", "create"]);
    assert!(
        err.contains("--name") || err.to_lowercase().contains("required"),
        "unexpected error: {err}"
    );
}

#[test]
fn invalid_scope_fails() {
    let err = run_err(&["key", "create", "--scope", "nonsense"]);
    assert!(
        err.contains("invalid value") || err.contains("nonsense"),
        "unexpected error: {err}"
    );
}
