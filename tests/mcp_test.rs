use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_mcp_handshake_and_tools_list() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ghgrab"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ghgrab mcp process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Send initialize request
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}"#;
    writeln!(stdin, "{}", init_req).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read initialize response
    let mut init_resp_line = String::new();
    reader
        .read_line(&mut init_resp_line)
        .expect("Failed to read from stdout");
    let init_resp: Value = serde_json::from_str(&init_resp_line).expect("Invalid JSON response");

    assert_eq!(init_resp["jsonrpc"], "2.0");
    assert_eq!(init_resp["id"], 1);
    assert_eq!(init_resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "rmcp");

    // 2. Send initialized notification
    let initialized_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    writeln!(stdin, "{}", initialized_notif).expect("Failed to write notification to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // 3. Send tools/list request
    let list_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    writeln!(stdin, "{}", list_req).expect("Failed to write tools/list to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read tools/list response
    let mut list_resp_line = String::new();
    reader
        .read_line(&mut list_resp_line)
        .expect("Failed to read tools/list from stdout");
    let list_resp: Value = serde_json::from_str(&list_resp_line).expect("Invalid JSON response");

    assert_eq!(list_resp["jsonrpc"], "2.0");
    assert_eq!(list_resp["id"], 2);

    let tools = list_resp["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name should be a string"))
        .collect();

    assert!(tool_names.contains(&"repo_tree"));
    assert!(tool_names.contains(&"download_files"));
    assert!(tool_names.contains(&"download_release"));
    assert!(tool_names.contains(&"search_repos"));
    assert!(tool_names.contains(&"read_file"));
    assert!(tool_names.contains(&"read_file_preview"));
    assert!(tool_names.contains(&"list_releases"));
    assert!(tool_names.contains(&"repo_info"));

    // Gracefully shutdown child by dropping stdin
    drop(stdin);
    let status = child.wait().expect("Failed to wait on child process");
    assert!(status.success());
}

#[test]
fn test_mcp_repo_info_on_actual_repo() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ghgrab"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ghgrab mcp process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Send initialize request
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}"#;
    writeln!(stdin, "{}", init_req).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    let mut init_resp_line = String::new();
    reader
        .read_line(&mut init_resp_line)
        .expect("Failed to read from stdout");

    // 2. Send initialized notification
    let initialized_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    writeln!(stdin, "{}", initialized_notif).expect("Failed to write notification to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // 3. Call repo_info tool
    let call_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"repo_info","arguments":{"url":"https://github.com/abhixdd/ghgrab"}}}"#;
    writeln!(stdin, "{}", call_req).expect("Failed to write tools/call to stdin");
    stdin.flush().expect("Failed to flush stdin");

    let mut call_resp_line = String::new();
    reader
        .read_line(&mut call_resp_line)
        .expect("Failed to read tools/call from stdout");
    let call_resp: Value = serde_json::from_str(&call_resp_line).expect("Invalid JSON response");

    assert_eq!(call_resp["jsonrpc"], "2.0");
    assert_eq!(call_resp["id"], 2);

    println!("Call response: {:?}", call_resp);

    // Assert either structured result or structured error (like rate limiting, auth issues, or offline)
    let result = &call_resp["result"];
    if result["isError"].as_bool().unwrap_or(false) {
        let err_msg = result["content"][0]["text"].as_str().unwrap_or_default();
        println!(
            "Info: MCP repo_info call returned structured error (tolerated in test): {}",
            err_msg
        );
    } else {
        // Successful response structure: MCP CallToolResult structured contains content
        let content_text = result["content"][0]["text"]
            .as_str()
            .expect("text should be present");
        let info: Value =
            serde_json::from_str(content_text).expect("nested text should be valid JSON");
        assert_eq!(info["owner"], "abhixdd");
        assert_eq!(info["repo"], "ghgrab");
        assert_eq!(info["platform"], "GitHub");
    }

    drop(stdin);
    let _ = child.wait();
}
