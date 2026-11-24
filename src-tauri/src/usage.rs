//! Plan usage per account: how much of each limit window (5 hours, 7 days…)
//! is used and when it resets, taken from what the CLIs report.
//!
//! * Claude Code passes `rate_limits` to its statusline command (Kilat's).
//! * Codex writes `token_count` events (with `rate_limits` and the context
//!   size) to the session's rollout file, which is read after each turn.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::store::{AccountUsage, UsageWindow};
use crate::util::{self, now_ms};

fn window_label(minutes: i64) -> String {
    match minutes {
        300 => "5 jam".into(),
        10080 => "7 hari".into(),
        m if m > 0 && m % 1440 == 0 => format!("{} hari", m / 1440),
        m if m > 0 && m % 60 == 0 => format!("{} jam", m / 60),
        m => format!("{m} menit"),
    }
}

fn claude_label(key: &str) -> String {
    match key {
        "five_hour" => "5 jam".into(),
        "seven_day" => "7 hari".into(),
        "seven_day_opus" => "7 hari (Opus)".into(),
        "seven_day_sonnet" => "7 hari (Sonnet)".into(),
        other => other.replace('_', " "),
    }
}

fn claude_window(label: String, order: usize, v: &Value) -> Option<(usize, UsageWindow)> {
    let used = ["used_percentage", "percent", "utilization"]
        .iter()
        .find_map(|k| v.get(*k).and_then(Value::as_f64))?;
    // "utilization" is 0-1 in the API headers, a percentage elsewhere.
    let used = if v.get("used_percentage").is_none() && v.get("percent").is_none() && used <= 1.0 {
        used * 100.0
    } else {
        used
    };
    Some((
        order,
        UsageWindow {
            label,
            used_percent: (used * 10.0).round() / 10.0,
            resets_at: v.get("resets_at").or_else(|| v.get("resetsAt")).and_then(util::to_ms),
        },
    ))
}

/// From the `rate_limits` Claude Code passes to its statusline command:
/// `{five_hour: {used_percentage, resets_at}, seven_day: {…}}`, or the
/// newer `{limits: [{kind, percent, resets_at}]}`.
pub fn from_claude(rl: &Value) -> Option<AccountUsage> {
    let mut windows: Vec<(usize, UsageWindow)> = vec![];
    if let Some(rows) = rl.get("limits").and_then(Value::as_array) {
        for r in rows {
            let kind = r.get("kind").and_then(Value::as_str).unwrap_or("");
            let (label, order) = match kind {
                "session" | "five_hour" => ("5 jam".to_string(), 0),
                "weekly_all" | "seven_day" => ("7 hari".to_string(), 1),
                other => {
                    let scope = r
                        .pointer("/scope/model/display_name")
                        .or_else(|| r.pointer("/scope/surface/display_name"))
                        .and_then(Value::as_str);
                    (scope.map(|s| format!("7 hari ({s})")).unwrap_or_else(|| other.replace('_', " ")), 2)
                }
            };
            windows.extend(claude_window(label, order, r));
        }
    }
    if let Some(obj) = rl.as_object() {
        for (k, v) in obj.iter().filter(|(_, v)| v.is_object()) {
            let order = match k.as_str() {
                "five_hour" => 0,
                "seven_day" => 1,
                _ => 2,
            };
            if windows.iter().any(|(_, w)| w.label == claude_label(k)) {
                continue;
            }
            windows.extend(claude_window(claude_label(k), order, v));
        }
    }
    if windows.is_empty() {
        return None;
    }
    windows.sort_by_key(|(o, _)| *o);
    Some(AccountUsage {
        updated_at: now_ms(),
        windows: windows.into_iter().map(|(_, w)| w).collect(),
    })
}

fn codex_window(v: &Value) -> Option<UsageWindow> {
    let used = v.get("used_percent")?.as_f64()?;
    let resets_at = v
        .get("resets_at")
        .and_then(Value::as_i64)
        .map(|s| s * 1000)
        .or_else(|| v.get("resets_in_seconds").and_then(Value::as_i64).map(|s| now_ms() + s * 1000));
    Some(UsageWindow {
        label: window_label(v.get("window_minutes").and_then(Value::as_i64).unwrap_or(0)),
        used_percent: used,
        resets_at,
    })
}

/// From a Codex `token_count` payload: (plan usage, context size json).
pub fn from_codex_token_count(p: &Value) -> (Option<AccountUsage>, Option<Value>) {
    let rl = &p["rate_limits"];
    let windows: Vec<UsageWindow> = ["primary", "secondary"]
        .iter()
        .filter_map(|k| codex_window(&rl[*k]))
        .collect();
    let usage = (!windows.is_empty()).then(|| AccountUsage { updated_at: now_ms(), windows });
    let info = &p["info"];
    let context = info["last_token_usage"]["input_tokens"].as_i64().map(|t| {
        json!({ "contextTokens": t, "contextWindow": info["model_context_window"].as_i64() })
    });
    (usage, context)
}

/// The rollout file of a Codex session (sessions/YYYY/MM/DD/rollout-…-<id>.jsonl).
pub fn find_rollout(root: &Path, session_id: &str) -> Option<PathBuf> {
    let suffix = format!("{session_id}.jsonl");
    let mut days: Vec<PathBuf> = vec![];
    let mut years: Vec<_> = fs::read_dir(root).ok()?.flatten().map(|e| e.path()).collect();
    years.sort();
    for y in years.iter().rev() {
        let mut months: Vec<_> = fs::read_dir(y).into_iter().flatten().flatten().map(|e| e.path()).collect();
        months.sort();
        for m in months.iter().rev() {
            let mut ds: Vec<_> = fs::read_dir(m).into_iter().flatten().flatten().map(|e| e.path()).collect();
            ds.sort();
            days.extend(ds.into_iter().rev());
            if days.len() > 120 {
                break;
            }
        }
    }
    days.iter().find_map(|d| {
        fs::read_dir(d)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().ends_with(&suffix)))
    })
}

/// When a limited account can be used again, if the usage says so.
pub fn reset_time(usage: &AccountUsage) -> Option<i64> {
    usage
        .windows
        .iter()
        .filter(|w| w.used_percent >= 99.5)
        .filter_map(|w| w.resets_at)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_statusline_limits() {
        let rl = json!({
            "seven_day": { "used_percentage": 53.0, "resets_at": 1795046400 },
            "five_hour": { "used_percentage": 41.2, "resets_at": 1794987600 }
        });
        let u = from_claude(&rl).unwrap();
        assert_eq!(u.windows[0].label, "5 jam");
        assert_eq!(u.windows[0].used_percent, 41.2);
        assert_eq!(u.windows[0].resets_at, Some(1794987600000));
        assert_eq!(u.windows[1].label, "7 hari");
        assert!(from_claude(&json!({})).is_none());
        let rows = json!({ "limits": [
            { "kind": "session", "percent": 100.0, "resets_at": "2026-11-18T03:13:34.911Z" }
        ]});
        let u = from_claude(&rows).unwrap();
        assert_eq!(u.windows[0].label, "5 jam");
        assert_eq!(reset_time(&u), Some(1_794_971_614_911));
    }

    #[test]
    fn parses_codex_token_count() {
        let p = json!({
            "type": "token_count",
            "info": { "last_token_usage": { "input_tokens": 225686 }, "model_context_window": 258400 },
            "rate_limits": {
                "primary": { "used_percent": 100.0, "window_minutes": 43200, "resets_at": 1796816971 },
                "secondary": null
            }
        });
        let (u, c) = from_codex_token_count(&p);
        let u = u.unwrap();
        assert_eq!(u.windows.len(), 1);
        assert_eq!(u.windows[0].label, "30 hari");
        assert_eq!(reset_time(&u), Some(1796816971000));
        assert_eq!(c.unwrap()["contextWindow"], 258400);
    }
}
