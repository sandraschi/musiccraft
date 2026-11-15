//! MusicCraft L2 Unified CLI + MCP 2024-11-05 Stdio Server.
//!
//! Stdout is 100% pure line-delimited JSON-RPC in `mcp` mode.
//! All diagnostics and logs go to `stderr`.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::{Parser, Subcommand};
use musiccraft_core::{
    command_catalog, dispatch, dispatch_on_project, resolve_project, Command, APP_VERSION,
};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "musiccraft-mcp",
    about = "MusicCraft Studio CLI & MCP Server (DAW, Vocal Synth, Stem Split, Mix Render)",
    version = APP_VERSION
)]
struct Cli {
    /// Run headless smoke verification and exit 0/1
    #[arg(long)]
    smoke: bool,

    /// Run as MCP stdio JSON-RPC server
    #[arg(long)]
    mcp: bool,

    /// Emit JSON output for CLI commands
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Sub>,
}

#[derive(Subcommand, Debug)]
enum Sub {
    /// Run MCP 2024-11-05 stdio JSON-RPC loop
    Mcp,
    /// Print the machine-readable command catalog
    Commands,
    /// Describe a single command ID and its JSON parameter schema
    Describe { id: String },
    /// Execute one or more commands (`id={json}`) on a project
    Run {
        #[arg(long)]
        project: Option<String>,
        #[arg(long = "cmd")]
        cmds: Vec<String>,
        #[arg(long)]
        save: Option<String>,
    },
    /// Song project lifecycle (`new`, `open`, `info`)
    Project {
        #[command(subcommand)]
        action: ProjectCmd,
    },
    /// Track management (`add`, `list`)
    Track {
        #[command(subcommand)]
        action: TrackCmd,
    },
    /// Stem separation (`split`)
    Stem {
        #[command(subcommand)]
        action: StemCmd,
    },
    /// Virtual vocal synthesizer (`synth`)
    Vocal {
        #[command(subcommand)]
        action: VocalCmd,
    },
    /// Offline mix rendering (`render`)
    Mix {
        #[command(subcommand)]
        action: MixCmd,
    },
    /// Handoff export (`stems`, `song`)
    Export {
        #[command(subcommand)]
        action: ExportCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCmd {
    New {
        #[arg(long, default_value = "Untitled Song")]
        name: String,
        #[arg(long, default_value_t = 120.0)]
        bpm: f64,
        #[arg(long, default_value = "C major")]
        key: String,
        #[arg(long)]
        out: Option<String>,
    },
    Open {
        path: String,
    },
    Info {
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum TrackCmd {
    Add {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "midi")]
        kind: String,
    },
    List {
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum StemCmd {
    Split {
        #[arg(long = "in")]
        in_wav: Option<String>,
        #[arg(long)]
        out_dir: String,
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum VocalCmd {
    Synth {
        #[arg(long)]
        line: String,
        #[arg(long, default_value = "OPERA")]
        voice: String,
        #[arg(long)]
        bpm: Option<f64>,
        #[arg(long)]
        base_midi: Option<u8>,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum MixCmd {
    Render {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        out: String,
    },
}

#[derive(Subcommand, Debug)]
enum ExportCmd {
    Stems {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        out_dir: String,
    },
    Song {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        out_dir: String,
    },
}

/// Pure function returning MCP `tools/list` payload.
pub fn tools_list() -> Value {
    let tools: Vec<Value> = command_catalog()
        .into_iter()
        .map(|spec| {
            json!({
                "name": spec.mcp_name,
                "description": format!("[{}] {}", spec.id, spec.description),
                "inputSchema": spec.params_schema
            })
        })
        .collect();
    json!(tools)
}

/// Pure function executing a single MCP tool call through `musiccraft_core::dispatch`.
pub async fn call_tool(name: &str, args: &Value) -> Value {
    match Command::from_name_and_params(name, args) {
        Ok(cmd) => dispatch(cmd).await,
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

fn handle_jsonrpc_line(trimmed: &str, rt: &tokio::runtime::Runtime) -> Option<Value> {
    let parsed: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            return Some(json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("Parse error: {e}") }
            }));
        }
    };

    // Notifications without "id" receive NO response.
    let id = parsed.get("id")?.clone();
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = parsed.get("params").cloned().unwrap_or_else(|| json!({}));

    let resp = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": { "name": "musiccraft", "version": APP_VERSION }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tools_list() }
        }),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let tool_out = rt.block_on(call_tool(name, &args));
            let is_err = tool_out
                .get("ok")
                .and_then(|v| v.as_bool())
                .map(|b| !b)
                .unwrap_or(false)
                || tool_out.get("error").is_some();
            let text = serde_json::to_string_pretty(&tool_out)
                .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": text }],
                    "isError": is_err
                }
            })
        }
        "resources/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": [
                    {
                        "uri": "musiccraft://commands",
                        "name": "MusicCraft Command Catalog",
                        "description": "Full machine-readable ArtCraft command bus catalog",
                        "mimeType": "application/json"
                    }
                ]
            }
        }),
        "resources/read" => {
            let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
            if uri == "musiccraft://commands" {
                let text = serde_json::to_string_pretty(&command_catalog())
                    .unwrap_or_else(|_| "[]".to_string());
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": text
                        }]
                    }
                })
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32602, "message": format!("Unknown resource URI: {uri}") }
                })
            }
        }
        other => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("Method not found: {other}") }
        }),
    };
    Some(resp)
}

fn run_mcp_stdio(rt: &tokio::runtime::Runtime) {
    eprintln!("[musiccraft-mcp] Starting MCP 2024-11-05 stdio loop");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let raw = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(resp) = handle_jsonrpc_line(trimmed, rt) {
            let _ = writeln!(stdout, "{resp}");
            let _ = stdout.flush();
        }
    }
}

fn run_smoke() -> std::result::Result<(), String> {
    let tmp = std::env::temp_dir().join("musiccraft_mcp_smoke");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let mut proj = resolve_project(None).map_err(|e| e.to_string())?;
    let v_res = dispatch_on_project(
        &mut proj,
        Command::VocalSynth {
            project_path: None,
            line: "aa ee oo ah".to_string(),
            voice: "OPERA".to_string(),
            bpm: Some(120.0),
            base_midi: Some(69),
            out_wav: Some(tmp.join("vocal.wav").to_string_lossy().to_string()),
        },
    );
    if v_res.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return Err(format!("vocal.synth failed: {v_res}"));
    }

    let mix_wav = tmp.join("mix.wav");
    let m_res = dispatch_on_project(
        &mut proj,
        Command::MixRender {
            project_path: None,
            out_wav: mix_wav.to_string_lossy().to_string(),
        },
    );
    if m_res.get("ok").and_then(|v| v.as_bool()) != Some(true) || !mix_wav.exists() {
        return Err(format!("mix.render failed: {m_res}"));
    }

    let handoff_dir = tmp.join("handoff");
    let e_res = dispatch_on_project(
        &mut proj,
        Command::ExportStems {
            project_path: None,
            out_dir: handoff_dir.to_string_lossy().to_string(),
        },
    );
    if e_res.get("ok").and_then(|v| v.as_bool()) != Some(true)
        || !handoff_dir.join("handoff.json").exists()
    {
        return Err(format!("export.stems failed: {e_res}"));
    }

    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}

fn print_value(val: &Value, as_json: bool) {
    if as_json {
        if let Ok(s) = serde_json::to_string_pretty(val) {
            println!("{s}");
        }
    } else if let Ok(s) = serde_json::to_string_pretty(val) {
        println!("{s}");
    }
}

fn main() {
    let cli = Cli::parse();
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[musiccraft-mcp] failed to initialize tokio runtime: {e}");
            std::process::exit(1);
        }
    };

    if cli.smoke {
        match run_smoke() {
            Ok(()) => {
                println!(
                    "{{\"ok\":true,\"smoke\":\"passed\",\"app\":\"musiccraft-mcp\",\"version\":\"{APP_VERSION}\"}}"
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("[musiccraft-mcp] smoke failed: {e}");
                std::process::exit(1);
            }
        }
    }

    if cli.mcp || matches!(cli.command, Some(Sub::Mcp)) || cli.command.is_none() {
        run_mcp_stdio(&rt);
        return;
    }

    let Some(sub) = cli.command else {
        return;
    };

    match sub {
        Sub::Mcp => run_mcp_stdio(&rt),
        Sub::Commands => {
            let val = json!({ "ok": true, "commands": command_catalog() });
            print_value(&val, cli.json);
        }
        Sub::Describe { id } => {
            let found = command_catalog()
                .into_iter()
                .find(|c| c.id == id || c.mcp_name == id);
            match found {
                Some(spec) => print_value(&json!({ "ok": true, "command": spec }), cli.json),
                None => {
                    print_value(
                        &json!({ "ok": false, "error": format!("unknown command '{id}'") }),
                        cli.json,
                    );
                    std::process::exit(1);
                }
            }
        }
        Sub::Run {
            project,
            cmds,
            save,
        } => {
            let mut proj = match resolve_project(project.as_deref()) {
                Ok(p) => p,
                Err(e) => {
                    print_value(&json!({ "ok": false, "error": e.to_string() }), cli.json);
                    std::process::exit(1);
                }
            };
            let mut results = Vec::new();
            for raw in cmds {
                let (name, params) = match raw.split_once('=') {
                    Some((k, v)) => {
                        let parsed: Value = serde_json::from_str(v).unwrap_or_else(|_| json!({}));
                        (k.trim(), parsed)
                    }
                    None => (raw.trim(), json!({})),
                };
                match Command::from_name_and_params(name, &params) {
                    Ok(cmd) => results.push(dispatch_on_project(&mut proj, cmd)),
                    Err(e) => results.push(json!({ "ok": false, "error": e.to_string() })),
                }
            }
            if let Some(save_path) = save {
                let _ = proj.save_to_file(&PathBuf::from(save_path));
            }
            print_value(&json!({ "ok": true, "results": results }), cli.json);
        }
        Sub::Project { action } => {
            let cmd = match action {
                ProjectCmd::New {
                    name,
                    bpm,
                    key,
                    out,
                } => Command::ProjectNew {
                    name,
                    bpm,
                    key,
                    out_path: out,
                },
                ProjectCmd::Open { path } => Command::ProjectOpen { path },
                ProjectCmd::Info { project } => Command::ProjectInfo { path: project },
            };
            let res = rt.block_on(dispatch(cmd));
            print_value(&res, cli.json);
        }
        Sub::Track { action } => {
            let cmd = match action {
                TrackCmd::Add {
                    project,
                    name,
                    kind,
                } => Command::TrackAdd {
                    project_path: project,
                    name,
                    kind,
                },
                TrackCmd::List { project } => Command::TrackList {
                    project_path: project,
                },
            };
            let res = rt.block_on(dispatch(cmd));
            print_value(&res, cli.json);
        }
        Sub::Stem { action } => {
            let cmd = match action {
                StemCmd::Split {
                    in_wav,
                    out_dir,
                    project,
                } => Command::StemSplit {
                    project_path: project,
                    in_wav,
                    out_dir,
                },
            };
            let res = rt.block_on(dispatch(cmd));
            print_value(&res, cli.json);
        }
        Sub::Vocal { action } => {
            let cmd = match action {
                VocalCmd::Synth {
                    line,
                    voice,
                    bpm,
                    base_midi,
                    out,
                    project,
                } => Command::VocalSynth {
                    project_path: project,
                    line,
                    voice,
                    bpm,
                    base_midi,
                    out_wav: out,
                },
            };
            let res = rt.block_on(dispatch(cmd));
            print_value(&res, cli.json);
        }
        Sub::Mix { action } => {
            let cmd = match action {
                MixCmd::Render { project, out } => Command::MixRender {
                    project_path: project,
                    out_wav: out,
                },
            };
            let res = rt.block_on(dispatch(cmd));
            print_value(&res, cli.json);
        }
        Sub::Export { action } => {
            let cmd = match action {
                ExportCmd::Stems { project, out_dir } => Command::ExportStems {
                    project_path: project,
                    out_dir,
                },
                ExportCmd::Song { project, out_dir } => Command::ExportSong {
                    project_path: project,
                    out_dir,
                },
            };
            let res = rt.block_on(dispatch(cmd));
            print_value(&res, cli.json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tools_list_and_jsonrpc_initialize() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        assert!(rt.is_ok());
        if let Ok(rt) = rt {
            let init = handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
                &rt,
            );
            assert!(init.is_some());
            if let Some(resp) = init {
                assert_eq!(
                    resp.pointer("/result/protocolVersion")
                        .and_then(|v| v.as_str()),
                    Some("2024-11-05")
                );
            }

            // Notifications must produce None
            let notif = handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                &rt,
            );
            assert!(notif.is_none());

            let list =
                handle_jsonrpc_line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, &rt);
            assert!(list.is_some());
            if let Some(resp) = list {
                let count = resp
                    .pointer("/result/tools")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                assert!(count >= 15);
            }

            let call = handle_jsonrpc_line(
                r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"musiccraft_status","arguments":{}}}"#,
                &rt,
            );
            assert!(call.is_some());
            if let Some(resp) = call {
                assert_eq!(
                    resp.pointer("/result/isError").and_then(|v| v.as_bool()),
                    Some(false)
                );
            }
        }
    }

    #[test]
    fn test_mcpb_manifest_matches_tools_list() {
        let manifest_raw = include_str!("../../../mcpb/manifest.json");
        let parsed = serde_json::from_str::<Value>(manifest_raw);
        assert!(parsed.is_ok(), "invalid mcpb/manifest.json");
        let manifest = parsed.unwrap_or(Value::Null);
        let manifest_names: Vec<String> = manifest
            .get("tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let served_names: Vec<String> = tools_list()
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        assert_eq!(manifest_names, served_names);
    }
}
