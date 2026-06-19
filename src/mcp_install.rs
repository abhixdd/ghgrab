use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

struct ClientInfo {
    name: &'static str,
    path: PathBuf,
    uses_servers_key: bool,
}

pub fn install_mcp_client(existing_token: Option<String>) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;

    let clients = [
        ClientInfo {
            name: "Claude Desktop",
            path: config_dir.join("Claude").join("claude_desktop_config.json"),
            uses_servers_key: false,
        },
        ClientInfo {
            name: "Cursor",
            path: home.join(".cursor").join("mcp.json"),
            uses_servers_key: false,
        },
        ClientInfo {
            name: "VS Code (Copilot)",
            path: home.join(".vscode").join("mcp.json"),
            uses_servers_key: true,
        },
        ClientInfo {
            name: "Windsurf",
            path: home
                .join(".codeium")
                .join("windsurf")
                .join("mcp_config.json"),
            uses_servers_key: false,
        },
    ];

    println!("\nDetecting MCP clients...");
    let mut detected = Vec::new();
    for (i, client) in clients.iter().enumerate() {
        let parent = client.path.parent().unwrap();
        let installed = parent.exists();
        if installed {
            detected.push(i);
            println!(
                "  {}. [Detected] {} ({})",
                detected.len(),
                client.name,
                client.path.display()
            );
        }
    }

    let choices = if detected.is_empty() {
        println!("  No MCP client installation folders were auto-detected.");
        println!("  Listing all supported clients for manual selection:\n");
        for (i, client) in clients.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, client.name, client.path.display());
        }
        clients
            .iter()
            .enumerate()
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    } else {
        detected
    };

    print!("\nSelect a client to configure (or type q to cancel): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("q") {
        println!("Installation cancelled.");
        return Ok(());
    }

    let index = trimmed
        .parse::<usize>()
        .map_err(|_| anyhow!("Invalid selection"))?;
    if index == 0 || index > choices.len() {
        return Err(anyhow!("Selection out of range"));
    }

    let selected_client = &clients[choices[index - 1]];

    let exe_path =
        std::env::current_exe().context("Failed to determine current executable path")?;
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| anyhow!("Invalid path characters in executable path"))?;

    let mut env_map = serde_json::Map::new();
    if let Some(tok) = existing_token {
        env_map.insert("GITHUB_TOKEN".to_string(), Value::String(tok));
    }

    let server_entry = json!({
        "command": exe_str,
        "args": ["mcp"],
        "env": Value::Object(env_map),
    });

    if let Some(parent) = selected_client.path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut config: Value = if selected_client.path.exists() {
        let content = fs::read_to_string(&selected_client.path)?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    let key = if selected_client.uses_servers_key {
        "servers"
    } else {
        "mcpServers"
    };

    if !config.is_object() {
        config = json!({});
    }
    let config_obj = config.as_object_mut().unwrap();
    if !config_obj.contains_key(key) {
        config_obj.insert(key.to_string(), json!({}));
    }

    let servers_map = config_obj
        .get_mut(key)
        .unwrap()
        .as_object_mut()
        .ok_or_else(|| anyhow!("Invalid config structure: '{}' key is not an object", key))?;

    if servers_map.contains_key("ghgrab") {
        print!(
            "⚠️  ghgrab is already configured in {}. Overwrite? [y/N]: ",
            selected_client.name
        );
        io::stdout().flush()?;
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm)?;
        let choice = confirm.trim().to_lowercase();
        if choice != "y" && choice != "yes" {
            println!("Cancelled.");
            return Ok(());
        }
    }

    servers_map.insert("ghgrab".to_string(), server_entry);

    let pretty_json = serde_json::to_string_pretty(&config)?;
    fs::write(&selected_client.path, pretty_json)?;

    println!(
        "\nghgrab MCP server configured successfully in {}!",
        selected_client.name
    );
    println!("   Config updated: {}", selected_client.path.display());
    println!("   Binary: {}", exe_str);
    println!(
        "\n   Please restart {} for changes to take effect.\n",
        selected_client.name
    );

    Ok(())
}
