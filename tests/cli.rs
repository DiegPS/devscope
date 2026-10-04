//! Real executable smoke tests. Only invoke commands that finish before config
//! loading; Windows KnownFolder APIs cannot be isolated with APPDATA overrides.
use std::process::Command;

fn ds(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ds"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_and_version_exit_successfully_with_the_installed_command_name() {
    let help = ds(&["--help"]);
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("Usage: ds"));
    assert!(text.contains("add-root"));
    assert!(text.contains("discover"));
    let version = ds(&["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        concat!("ds ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn invalid_arguments_return_nonzero_and_explain_the_error_on_stderr() {
    for args in [
        &["list", "--unknown"][..],
        &["note", "only-project"],
        &["status"],
        &["add-root"],
    ] {
        let result = ds(args);
        assert!(!result.status.success(), "{args:?}");
        assert!(result.stdout.is_empty(), "{args:?}");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("error:"));
        assert!(error.contains("Usage:"));
    }
}

#[test]
fn subcommand_help_is_available_without_loading_configuration() {
    for command in [
        "scan",
        "list",
        "add-root",
        "remove-root",
        "roots",
        "note",
        "status",
        "config",
        "open",
        "discover",
    ] {
        let result = ds(&[command, "--help"]);
        assert!(result.status.success(), "{command}");
        assert!(result.stderr.is_empty());
        let output = String::from_utf8(result.stdout).unwrap();
        let usage = output
            .lines()
            .find_map(|line| line.strip_prefix("Usage: "))
            .unwrap();
        let mut tokens = usage.split_whitespace();
        assert!(matches!(tokens.next(), Some("ds" | "ds.exe")));
        assert_eq!(tokens.next(), Some(command));
    }
}
