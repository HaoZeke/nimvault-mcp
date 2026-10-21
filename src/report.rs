//! Structured readings of nimvault's line-oriented reports.
//!
//! Tool results are these structs. File bodies never go in them.

use schemars::JsonSchema;
use serde::Serialize;

use crate::policy::agent_view;

fn redact(s: &str) -> String {
    let t = agent_view(s);
    let t = t.trim();
    if t.is_empty() {
        "command failed".into()
    } else {
        t.to_string()
    }
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('\u{1b}') {
        out.push_str(&rest[..start]);
        let after = &rest[start + '\u{1b}'.len_utf8()..];
        if let Some(stripped) = after.strip_prefix('[') {
            if let Some(end) = stripped.find('m') {
                rest = &stripped[end + 1..];
                continue;
            }
        }
        out.push('\u{1b}');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn combined(stdout: &str, stderr: &str) -> String {
    strip_ansi(&format!("{stdout}\n{stderr}"))
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct StatusEntry {
    /// in-sync, modified, missing, no-blob, or error.
    pub state: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct StatusReport {
    pub command: String,
    /// The command ran. A modified or missing file is still ok.
    pub ok: bool,
    pub empty: bool,
    pub entries: Vec<StatusEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct WhoEntry {
    pub path: String,
    pub recipients: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct WhoReport {
    pub command: String,
    pub ok: bool,
    pub empty: bool,
    pub entries: Vec<WhoEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct CheckReport {
    pub command: String,
    /// False when any entry disagrees with the manifest.
    pub ok: bool,
    pub empty: bool,
    pub checked: u32,
    pub problems: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct McpReadiness {
    pub version: String,
    pub mutate: bool,
    pub read_only: bool,
    pub cli_ok: bool,
    pub cli: String,
    pub lib_loaded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lib_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq, Default)]
pub struct VaultDoctor {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct DoctorReport {
    pub command: String,
    pub ok: bool,
    pub repo_resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    pub problems: Vec<String>,
    pub mcp: McpReadiness,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct WriteReport {
    pub command: String,
    pub ok: bool,
    pub repo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rekey: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WriteReport {
    pub fn success(
        command: &str,
        repo: &str,
        path: Option<String>,
        force: Option<bool>,
        rekey: Option<bool>,
    ) -> Self {
        Self {
            command: command.into(),
            ok: true,
            repo: repo.into(),
            path,
            force,
            rekey,
            error: None,
        }
    }

    pub fn failure(
        command: &str,
        repo: &str,
        path: Option<String>,
        force: Option<bool>,
        rekey: Option<bool>,
        err: &str,
    ) -> Self {
        Self {
            command: command.into(),
            ok: false,
            repo: repo.into(),
            path,
            force,
            rekey,
            error: Some(redact(err)),
        }
    }
}

const STATUS_STATES: &[&str] = &["in-sync", "modified", "missing", "no-blob", "error"];

fn status_line(t: &str) -> Option<StatusEntry> {
    let t = t.trim();
    let rest = t.strip_prefix('[')?;
    let (state, path) = rest.split_once(']')?;
    if !STATUS_STATES.contains(&state) {
        return None;
    }
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some(StatusEntry {
        state: state.to_string(),
        path: path.to_string(),
    })
}

pub fn parse_status(stdout: &str, stderr: &str, cli_ok: bool) -> StatusReport {
    let text = combined(stdout, stderr);
    let mut entries = Vec::new();
    let mut empty = false;
    for line in text.lines() {
        let t = line.trim();
        if t == "vault is empty" {
            empty = true;
            continue;
        }
        if let Some(e) = status_line(t) {
            entries.push(e);
        }
    }
    let failed_entry = entries.iter().any(|e| e.state == "error");
    let ok = cli_ok && !failed_entry;
    let error = if ok {
        None
    } else if !cli_ok {
        Some(redact(&text))
    } else {
        None
    };
    StatusReport {
        command: "status".into(),
        ok,
        empty: empty && entries.is_empty(),
        entries,
        error,
    }
}

pub fn parse_who(stdout: &str, stderr: &str, cli_ok: bool) -> WhoReport {
    let text = combined(stdout, stderr);
    let mut entries = Vec::new();
    let mut empty = false;
    for line in text.lines() {
        let t = line.trim();
        if t == "vault is empty" {
            empty = true;
            continue;
        }
        let Some((path, recips)) = t.split_once('\t') else {
            continue;
        };
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        let recipients = recips
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        entries.push(WhoEntry {
            path: path.to_string(),
            recipients,
        });
    }
    WhoReport {
        command: "who".into(),
        ok: cli_ok,
        empty: empty && entries.is_empty(),
        entries,
        error: if cli_ok { None } else { Some(redact(&text)) },
    }
}

fn check_summary(t: &str) -> Option<(u32, bool)> {
    let t = t.trim().trim_end_matches('.');
    if let Some(rest) = t.strip_suffix(" entries consistent with the manifest") {
        return rest.trim().parse::<u32>().ok().map(|n| (n, true));
    }
    if let Some(rest) = t.strip_suffix(" entries consistent") {
        return rest.trim().parse::<u32>().ok().map(|n| (n, true));
    }
    let marker = " of ";
    if let Some(idx) = t.find(marker) {
        let after = &t[idx + marker.len()..];
        if after.contains("inconsistent") {
            if let Some(nstr) = after.split_whitespace().next() {
                if let Ok(n) = nstr.parse::<u32>() {
                    return Some((n, false));
                }
            }
        }
    }
    None
}

fn ignorable(t: &str) -> bool {
    let t = t.trim();
    t.is_empty()
        || t.chars().all(|c| c == '-')
        || t.starts_with("Checking vault")
        || t.starts_with("Vault status")
        || t.starts_with("Sealing vault")
        || t.starts_with("Unsealing")
        || t.starts_with("Rewrapping")
        || t.starts_with("Rekeying")
        || t.starts_with("Adding ")
}

pub fn parse_check(stdout: &str, stderr: &str, cli_ok: bool) -> CheckReport {
    let text = combined(stdout, stderr);
    let mut problems = Vec::new();
    let mut empty = false;
    let mut checked = 0u32;
    let mut summary = None;
    let mut consistent: Option<bool> = None;
    for line in text.lines() {
        let t = line.trim();
        if ignorable(t) {
            continue;
        }
        if t == "vault is empty" {
            empty = true;
            continue;
        }
        if let Some((n, good)) = check_summary(t) {
            checked = n;
            consistent = Some(good);
            summary = Some(t.to_string());
            continue;
        }
        problems.push(t.to_string());
    }
    let ok = if empty && problems.is_empty() {
        true
    } else if consistent == Some(true) && problems.is_empty() {
        true
    } else if consistent == Some(false) || !problems.is_empty() {
        false
    } else {
        cli_ok && problems.is_empty()
    };
    let error = if ok || !problems.is_empty() {
        None
    } else {
        Some(redact(&text))
    };
    CheckReport {
        command: "check".into(),
        ok,
        empty: empty && problems.is_empty() && consistent.is_none(),
        checked,
        problems,
        summary,
        error,
    }
}

fn field(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(&prefix) {
            let v = rest.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn parse_vault_doctor(stdout: &str, stderr: &str, cli_ok: bool) -> VaultDoctor {
    let text = combined(stdout, stderr);
    let mut problems = Vec::new();
    let mut in_problems = false;
    for line in text.lines() {
        let t = line.trim();
        if t == "problems:" {
            in_problems = true;
            continue;
        }
        if in_problems {
            if t.is_empty() || ignorable(t) {
                continue;
            }
            if !line.starts_with([' ', '\t']) && t.contains(':') {
                in_problems = false;
            } else {
                problems.push(t.to_string());
                continue;
            }
        }
    }
    if problems.is_empty() && !cli_ok {
        let t = text.trim();
        if !t.is_empty() {
            problems.push(t.to_string());
        }
    }
    let ok = cli_ok && problems.is_empty();
    VaultDoctor {
        ok,
        vault: field(&text, "vault"),
        backend: field(&text, "backend"),
        recipient: field(&text, "recipient"),
        gpg: field(&text, "gpg"),
        age: field(&text, "age"),
        manifest: field(&text, "manifest"),
        check: field(&text, "check"),
        problems,
    }
}

pub fn assemble_doctor(mcp: McpReadiness, repo_resolved: bool, vault: VaultDoctor) -> DoctorReport {
    let ok = if repo_resolved {
        mcp.cli_ok && vault.ok
    } else {
        mcp.cli_ok && vault.problems.is_empty()
    };
    DoctorReport {
        command: "doctor".into(),
        ok,
        repo_resolved,
        vault: vault.vault,
        backend: vault.backend,
        recipient: vault.recipient,
        gpg: vault.gpg,
        age: vault.age,
        manifest: vault.manifest,
        check: vault.check,
        problems: vault.problems,
        mcp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_strips_ansi_and_keeps_states() {
        let raw = "\n\u{1b}[36m\u{1b}[1m  Vault status\u{1b}[0m\n\u{1b}[32m  [in-sync]   secrets/a.txt\u{1b}[0m\n\u{1b}[31m  [modified]  secrets/b.txt\u{1b}[0m\n";
        let r = parse_status(raw, "", true);
        assert!(r.ok);
        assert!(!r.empty);
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[0].state, "in-sync");
        assert_eq!(r.entries[0].path, "secrets/a.txt");
        assert_eq!(r.entries[1].state, "modified");
        assert!(r.error.is_none());
    }

    #[test]
    fn status_empty() {
        let r = parse_status("vault is empty\n", "", true);
        assert!(r.ok && r.empty && r.entries.is_empty());
    }

    #[test]
    fn who_splits_recipients() {
        let r = parse_who("secrets/a.txt\tAAAA,BBBB\nvault is empty\n", "", true);
        assert!(r.ok);
        assert!(!r.empty);
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].recipients, vec!["AAAA", "BBBB"]);
    }

    #[test]
    fn check_consistent_and_not() {
        let good = parse_check(
            "\n  Checking vault ...\n\n1 entries consistent with the manifest.\n",
            "",
            true,
        );
        assert!(good.ok);
        assert_eq!(good.checked, 1);
        assert!(good.problems.is_empty());

        let bad = parse_check(
            "Checking vault ...\n",
            "  blob does not match the manifest for secrets/a.txt\n1 of 1 entries are inconsistent (seal, or restore the missing blob)\n",
            false,
        );
        assert!(!bad.ok);
        assert_eq!(bad.checked, 1);
        assert_eq!(bad.problems.len(), 1);
        assert!(bad.problems[0].contains("secrets/a.txt"));
    }

    #[test]
    fn doctor_fields_and_problems() {
        let text = "\
vault:     /tmp/repo
backend:   gpg
recipient: ABCDEF
gpg:       ok
manifest:  manifest.gpg
check:     1 entries consistent
";
        let d = parse_vault_doctor(text, "", true);
        assert!(d.ok);
        assert_eq!(d.vault.as_deref(), Some("/tmp/repo"));
        assert_eq!(d.backend.as_deref(), Some("gpg"));
        assert_eq!(d.recipient.as_deref(), Some("ABCDEF"));
        assert_eq!(d.gpg.as_deref(), Some("ok"));
        assert_eq!(d.manifest.as_deref(), Some("manifest.gpg"));
        assert_eq!(d.check.as_deref(), Some("1 entries consistent"));

        let bad = parse_vault_doctor(
            "vault:     /tmp/repo\nbackend:   gpg\nrecipient: ABCDEF\nproblems:\n  gpg is not on PATH\n",
            "",
            false,
        );
        assert!(!bad.ok);
        assert_eq!(bad.problems, vec!["gpg is not on PATH".to_string()]);
    }

    #[test]
    fn write_failure_drops_token_line() {
        let secret = "sk-abcdefghijklmnopqrstuvwxyz012345";
        let r = WriteReport::failure(
            "add",
            "/tmp/repo",
            Some("secret.txt".into()),
            None,
            None,
            &format!("boom {secret}"),
        );
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains(secret), "{s}");
        assert!(!r.ok);
        let ok = WriteReport::success("seal", "/tmp/repo", None, Some(false), None);
        let s = serde_json::to_string(&ok).unwrap();
        assert!(!s.contains("error"));
        assert!(s.contains("\"force\":false"));
    }
}
