use serde_json::Value;
use std::process::Command;

fn run_agent_command(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_ghgrab"))
        .args(args)
        .output()
        .expect("failed to execute ghgrab binary");

    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON")
}

#[test]
fn agent_tree_invalid_url_returns_json_error() {
    let payload = run_agent_command(&["agent", "tree", "not-a-url"]);

    assert_eq!(payload["api_version"], "1");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["command"], "tree");
    assert_eq!(payload["error"]["code"], "invalid_url");
    assert!(payload["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("Invalid URL"));
}

#[test]
fn agent_download_conflicting_repo_flags_return_json_error() {
    let payload = run_agent_command(&[
        "agent",
        "download",
        "https://github.com/rust-lang/rust",
        "README.md",
        "--repo",
    ]);

    assert_eq!(payload["api_version"], "1");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["command"], "download");
    assert_eq!(payload["error"]["code"], "invalid_arguments");
    assert!(payload["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains("--repo cannot be combined"));
}
