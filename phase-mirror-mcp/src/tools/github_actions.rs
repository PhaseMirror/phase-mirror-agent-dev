use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Command;

pub fn execute_cli_command(cmd: &str, args: &[&str]) -> Value {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                json!({
                    "isError": false,
                    "content": [{ "type": "text", "text": format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr) }]
                })
            } else {
                json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": format!("Command failed with exit code {}.\nSTDOUT:\n{}\nSTDERR:\n{}", output.status.code().unwrap_or(-1), stdout, stderr) }]
                })
            }
        }
        Err(e) => json!({
            "isError": true,
            "content": [{ "type": "text", "text": format!("Failed to execute {}: {}", cmd, e) }]
        }),
    }
}

pub fn handle_git_local(args: &Value) -> Value {
    if let Some(subcmd_args) = args.get("args").and_then(|a| a.as_array()) {
        let string_args: Vec<String> = subcmd_args.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
        let str_args: Vec<&str> = string_args.iter().map(|s| s.as_str()).collect();
        execute_cli_command("git", &str_args)
    } else {
         json!({
            "isError": true,
            "content": [{ "type": "text", "text": "Missing 'args' array parameter" }]
        })
    }
}

pub fn handle_github_pr(args: &Value) -> Value {
    if let Some(subcmd_args) = args.get("args").and_then(|a| a.as_array()) {
        let string_args: Vec<String> = subcmd_args.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
        let mut str_args: Vec<&str> = vec!["pr"];
        str_args.extend(string_args.iter().map(|s| s.as_str()));
        execute_cli_command("gh", &str_args)
    } else {
         json!({
            "isError": true,
            "content": [{ "type": "text", "text": "Missing 'args' array parameter" }]
        })
    }
}

pub fn handle_github_issue(args: &Value) -> Value {
    if let Some(subcmd_args) = args.get("args").and_then(|a| a.as_array()) {
        let string_args: Vec<String> = subcmd_args.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
        let mut str_args: Vec<&str> = vec!["issue"];
        str_args.extend(string_args.iter().map(|s| s.as_str()));
        execute_cli_command("gh", &str_args)
    } else {
         json!({
            "isError": true,
            "content": [{ "type": "text", "text": "Missing 'args' array parameter" }]
        })
    }
}

pub fn handle_github_repo(args: &Value) -> Value {
    if let Some(subcmd_args) = args.get("args").and_then(|a| a.as_array()) {
        let string_args: Vec<String> = subcmd_args.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
        let mut str_args: Vec<&str> = vec!["repo"];
        str_args.extend(string_args.iter().map(|s| s.as_str()));
        execute_cli_command("gh", &str_args)
    } else {
         json!({
            "isError": true,
            "content": [{ "type": "text", "text": "Missing 'args' array parameter" }]
        })
    }
}
