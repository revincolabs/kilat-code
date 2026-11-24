//! Bridge from the agent CLIs back to Kilat.
//!
//! Claude Code runs Kilat's own exe as its hook and statusline command
//! (`kilat --kilat-hook`, `kilat --kilat-statusline`, see launch.rs). That
//! short-lived process forwards the JSON it gets on stdin to the running app
//! over a localhost socket whose address and key are in the agent's
//! environment, then prints the app's reply (the statusline text).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Weak};
use std::time::Duration;

use serde_json::{json, Value};

use super::AgentManager;

pub const ENV_ADDR: &str = "KILAT_HOOK_ADDR";
pub const ENV_KEY: &str = "KILAT_HOOK_KEY";
pub const ENV_THREAD: &str = "KILAT_THREAD";
/// Which agent process of the thread sent it (stale processes are ignored).
pub const ENV_GEN: &str = "KILAT_GEN";

pub const FLAG_HOOK: &str = "--kilat-hook";
pub const FLAG_STATUS: &str = "--kilat-statusline";

#[derive(Clone)]
pub struct HookServer {
    pub addr: String,
    pub key: String,
}

/// Listen on 127.0.0.1 (random port). Each connection carries one message.
pub fn serve(mgr: Weak<AgentManager>) -> std::io::Result<HookServer> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?.to_string();
    let key = uuid::Uuid::new_v4().simple().to_string();
    let k = key.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mgr = mgr.clone();
            let key = k.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, &key, mgr);
            });
        }
    });
    Ok(HookServer { addr, key })
}

fn handle(stream: TcpStream, key: &str, mgr: Weak<AgentManager>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).take(4 * 1024 * 1024).read_line(&mut line)?;
    let msg: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
    if msg["key"].as_str() != Some(key) {
        return Ok(());
    }
    let Some(mgr) = mgr.upgrade() else { return Ok(()) };
    let thread = msg["thread"].as_str().unwrap_or("");
    let gen = msg["gen"].as_str().and_then(|g| g.parse::<u64>().ok()).unwrap_or(0);
    let kind = msg["kind"].as_str().unwrap_or("");
    let reply = on_message(&mgr, thread, gen, kind, &msg["data"]).unwrap_or_default();
    let mut s = stream;
    s.write_all(reply.as_bytes())?;
    s.write_all(b"\n")?;
    s.flush()
}

fn on_message(mgr: &Arc<AgentManager>, thread: &str, gen: u64, kind: &str, data: &Value) -> Option<String> {
    match kind {
        "hook" => {
            mgr.on_claude_hook(thread, gen, data);
            None
        }
        "status" => Some(mgr.on_statusline(thread, gen, data)),
        _ => None,
    }
}

/// `kilat --kilat-hook` / `kilat --kilat-statusline`: run as a CLI helper
/// instead of the app. Returns the exit code, or None for a normal start.
pub fn cli() -> Option<i32> {
    let flag = std::env::args().nth(1)?;
    let kind = match flag.as_str() {
        FLAG_HOOK => "hook",
        FLAG_STATUS => "status",
        _ => return None,
    };
    let mut input = String::new();
    let _ = std::io::stdin().take(4 * 1024 * 1024).read_to_string(&mut input);
    let data: Value = serde_json::from_str(input.trim()).unwrap_or(Value::Null);
    let reply = send(kind, data);
    if kind == "status" {
        let text = reply.filter(|r| !r.is_empty()).unwrap_or_else(|| "⚡ Kilat".into());
        let mut out = std::io::stdout();
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    }
    Some(0)
}

fn send(kind: &str, data: Value) -> Option<String> {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let addr: SocketAddr = env(ENV_ADDR).parse().ok()?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_millis(1500)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(4))).ok()?;
    s.set_write_timeout(Some(Duration::from_secs(4))).ok()?;
    let msg = json!({
        "key": env(ENV_KEY),
        "thread": env(ENV_THREAD),
        "gen": env(ENV_GEN),
        "kind": kind,
        "data": data,
    });
    let mut line = msg.to_string();
    line.push('\n');
    s.write_all(line.as_bytes()).ok()?;
    s.flush().ok()?;
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply).ok()?;
    Some(reply.trim_end_matches(['\r', '\n']).to_string())
}
