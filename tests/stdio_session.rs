//! Drive nimvault-mcp over stdio against a temp vault and a throwaway GPG key.

mod common;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SECRET_TOKEN: &str = "sk-nimvaultmcpstdioabcdefghijklmnopqrstuvwxyz012345";
const SECRET_PLAIN: &str = "nimvault-mcp-stdio-fixture-7f3a9c1e";
const SECRET_EDIT: &str = "modified-nimvault-mcp-stdio-fixture-bb91";

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new(label: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("nv_mcp_stdio_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = Command::new("gpgconf")
            .args(["--kill", "gpg-agent"])
            .env("GNUPGHOME", self.root.join("gpg"))
            .status();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn lock_dir(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

fn nimvault() -> PathBuf {
    if let Some(p) = std::env::var_os("NIMVAULT_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let c = dir.join("nimvault");
            if c.is_file() {
                return c;
            }
        }
    }
    panic!("nimvault CLI is required (NIMVAULT_BIN or PATH)");
}

fn gpg_home(dir: &Path) -> String {
    lock_dir(dir);
    fs::write(dir.join("gpg.conf"), "batch\nno-tty\n").unwrap();
    fs::write(dir.join("gpg-agent.conf"), "allow-loopback-pinentry\n").unwrap();
    let script = dir.join("keygen.txt");
    fs::write(
        &script,
        "\
%no-protection
Key-Type: RSA
Key-Length: 2048
Subkey-Type: RSA
Subkey-Length: 2048
Name-Real: NimVault MCP Test
Name-Email: nimvault-mcp-test@example.invalid
Expire-Date: 0
%commit
",
    )
    .unwrap();
    let out = Command::new("gpg")
        .env("GNUPGHOME", dir)
        .args(["--batch", "--gen-key"])
        .arg(&script)
        .output()
        .expect("gpg");
    assert!(
        out.status.success(),
        "gpg keygen failed\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let listed = Command::new("gpg")
        .env("GNUPGHOME", dir)
        .args([
            "--list-keys",
            "--keyid-format",
            "long",
            "--with-colons",
            "nimvault-mcp-test@example.invalid",
        ])
        .output()
        .unwrap();
    assert!(
        listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    for line in String::from_utf8_lossy(&listed.stdout).lines() {
        if line.starts_with("pub:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() > 4 && !parts[4].is_empty() {
                return parts[4].to_string();
            }
        }
    }
    panic!("throwaway key id missing");
}

struct Session {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    stderr: Arc<Mutex<String>>,
    transcript: String,
    next_id: i64,
}

impl Session {
    fn spawn(repo: &Path, gpg: &Path, audit: &Path) -> Self {
        let scratch = repo
            .parent()
            .unwrap_or(repo)
            .join(format!("cwd-{}", std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        fs::create_dir_all(&scratch).unwrap();
        let mut child = Command::new(common::mcp_bin())
            .current_dir(&scratch)
            .env("GNUPGHOME", gpg)
            .env("NIMVAULT_BIN", nimvault())
            .env("NIMVAULT_MCP_ALLOW_MUTATE", "1")
            .env("NIMVAULT_MCP_AUDIT_LOG", audit)
            .env_remove("NIMVAULT_LIB")
            .env_remove("NIMVAULT_VAULT_REPO")
            .env_remove("NIMVAULT_DEFAULT_REPO")
            .env_remove("NIMVAULT_GPG_RECIPIENT")
            .env_remove("NIMVAULT_MCP_READ_ONLY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn nimvault-mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr_pipe = child.stderr.take().unwrap();
        let stderr = Arc::new(Mutex::new(String::new()));
        let err_buf = stderr.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(stderr_pipe);
            let mut line = String::new();
            while r.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(mut g) = err_buf.lock() {
                    g.push_str(&line);
                }
                line.clear();
            }
        });
        let mut s = Self {
            child,
            stdin,
            stdout,
            stderr,
            transcript: String::new(),
            next_id: 1,
        };
        s.send(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"nimvault-mcp-test","version":"0"}}}"#,
        );
        let init = s.read_id(1);
        assert!(init.get("result").is_some(), "initialize: {init}");
        s.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        s.next_id = 2;
        let _ = repo;
        let _ = scratch;
        s
    }

    fn send(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_id(&mut self, id: i64) -> serde_json::Value {
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        let mut line = String::new();
        loop {
            if std::time::Instant::now() > deadline {
                panic!(
                    "timeout waiting for id {id}\nstderr:\n{}\ntranscript:\n{}",
                    self.stderr.lock().unwrap(),
                    self.transcript
                );
            }
            line.clear();
            let n = self.stdout.read_line(&mut line).unwrap_or(0);
            if n == 0 {
                panic!(
                    "server closed stdout\nstderr:\n{}",
                    self.stderr.lock().unwrap()
                );
            }
            self.transcript.push_str(&line);
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
                continue;
            };
            if v.get("id").and_then(|x| x.as_i64()) == Some(id) {
                return v;
            }
        }
    }

    fn call(&mut self, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        });
        self.send(&msg.to_string());
        self.read_id(id)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn structured(resp: &serde_json::Value) -> serde_json::Value {
    let result = resp.get("result").cloned().unwrap_or_else(|| resp.clone());
    if let Some(v) = result.get("structuredContent") {
        return v.clone();
    }
    if let Some(text) = result.pointer("/content/0/text").and_then(|t| t.as_str()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
            return v;
        }
    }
    panic!("no structured content: {resp}");
}

fn assert_clean(blob: &str) {
    assert!(!blob.contains(SECRET_TOKEN), "{blob}");
    assert!(!blob.contains(SECRET_PLAIN), "{blob}");
    assert!(!blob.contains(SECRET_EDIT), "{blob}");
}

#[test]
fn stdio_roundtrip_hides_plaintext() {
    let _ = Command::new("gpg").arg("--version").output().expect("gpg");
    let tree = TempTree::new("vault");
    let gpg = tree.root.join("gpg");
    fs::create_dir_all(&gpg).unwrap();
    let key = gpg_home(&gpg);
    let repo = tree.root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    assert!(Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success());
    let nv = nimvault();
    let init = Command::new(&nv)
        .args(["init", "--recipient", &key, "--vault"])
        .arg(&repo)
        .env("GNUPGHOME", &gpg)
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "init\n{}{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );
    let secret_path = repo.join("secret.txt");
    let path_s = secret_path.to_string_lossy().into_owned();
    let body = format!("{SECRET_TOKEN}\n{SECRET_PLAIN}\n");
    fs::write(&secret_path, &body).unwrap();

    let audit = tree.root.join("audit.log");
    let mut session = Session::spawn(&repo, &gpg, &audit);
    let repo_s = repo.to_string_lossy().to_string();

    let id = session.next_id;
    session.next_id += 1;
    session.send(&format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/list","params":{{}}}}"#
    ));
    let listed = session.read_id(id);
    let tools = listed
        .pointer("/result/tools")
        .and_then(|t| t.as_array())
        .expect("tools");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    for required in [
        "nimvault_status",
        "nimvault_who",
        "nimvault_check",
        "nimvault_doctor",
        "nimvault_add",
        "nimvault_seal",
        "nimvault_rotate",
        "nimvault_unseal",
    ] {
        assert!(names.contains(&required), "missing {required} in {names:?}");
    }
    assert!(!names.iter().any(|n| n.contains("lock")), "{names:?}");
    assert!(
        !names
            .iter()
            .any(|n| *n == "nimvault_get" || n.ends_with("_get")),
        "{names:?}"
    );
    for name in [
        "nimvault_status",
        "nimvault_who",
        "nimvault_check",
        "nimvault_doctor",
        "nimvault_add",
        "nimvault_seal",
        "nimvault_rotate",
    ] {
        let tool = tools
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(name))
            .unwrap();
        assert!(tool.get("outputSchema").is_some(), "{name} schema: {tool}");
    }

    let seal_missing = session.call("nimvault_seal", serde_json::json!({"repo_path": &repo_s}));
    let seal_blob = seal_missing.to_string();
    let rejected = seal_missing.get("error").is_some()
        || seal_missing
            .pointer("/result/isError")
            .and_then(|v| v.as_bool())
            == Some(true);
    assert!(rejected, "{seal_missing}");
    assert!(seal_blob.contains("force"), "{seal_missing}");

    let added = structured(&session.call(
        "nimvault_add",
        serde_json::json!({
            "path": path_s,
            "repo_path": &repo_s,
            "no_gitignore": true
        }),
    ));
    assert_eq!(
        added.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{added}"
    );
    assert_eq!(added.get("command").and_then(|v| v.as_str()), Some("add"));
    assert!(!added.to_string().contains("[redacted"), "{added}");

    let sealed = structured(&session.call(
        "nimvault_seal",
        serde_json::json!({"repo_path": &repo_s, "force": false}),
    ));
    assert_eq!(
        sealed.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{sealed}"
    );
    assert_eq!(sealed.get("force").and_then(|v| v.as_bool()), Some(false));
    assert!(!sealed.to_string().contains("[redacted"), "{sealed}");

    let status =
        structured(&session.call("nimvault_status", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        status.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{status}"
    );
    let states: Vec<&str> = status["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["state"].as_str().unwrap())
        .collect();
    assert_eq!(states, vec!["in-sync"], "{status}");

    let who = structured(&session.call("nimvault_who", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(who.get("ok").and_then(|v| v.as_bool()), Some(true), "{who}");
    let recips = who["entries"][0]["recipients"]
        .as_array()
        .expect("recipients");
    assert!(
        recips.iter().any(|r| r.as_str() == Some(key.as_str())),
        "{who} key {key}"
    );

    let check =
        structured(&session.call("nimvault_check", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        check.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{check}"
    );
    assert!(check["checked"].as_u64().unwrap() >= 1, "{check}");

    let doctor =
        structured(&session.call("nimvault_doctor", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        doctor.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{doctor}"
    );
    assert_eq!(doctor.get("backend").and_then(|v| v.as_str()), Some("gpg"));
    assert_eq!(doctor.get("gpg").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(
        doctor.get("recipient").and_then(|v| v.as_str()),
        Some(key.as_str())
    );
    assert!(
        doctor
            .get("mcp")
            .and_then(|m| m.get("cli_ok"))
            .and_then(|v| v.as_bool())
            == Some(true)
    );

    let wrapped = structured(&session.call(
        "nimvault_rotate",
        serde_json::json!({"repo_path": &repo_s, "rekey": false}),
    ));
    assert_eq!(
        wrapped.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{wrapped}"
    );
    assert_eq!(wrapped.get("rekey").and_then(|v| v.as_bool()), Some(false));

    let rekeyed = structured(&session.call(
        "nimvault_rotate",
        serde_json::json!({"repo_path": &repo_s, "rekey": true}),
    ));
    assert_eq!(
        rekeyed.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{rekeyed}"
    );
    assert_eq!(rekeyed.get("rekey").and_then(|v| v.as_bool()), Some(true));
    assert!(!rekeyed.to_string().contains("[redacted"), "{rekeyed}");

    let blob = fs::read_dir(repo.join(".vault"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            p.is_file()
                && name.ends_with(".gpg")
                && name != "manifest.gpg"
                && !name.starts_with("keys")
        })
        .expect("blob");
    let saved = fs::read(&blob).unwrap();
    let mut tampered = saved.clone();
    tampered.push(b'x');
    fs::write(&blob, &tampered).unwrap();
    let bad =
        structured(&session.call("nimvault_check", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        bad.get("ok").and_then(|v| v.as_bool()),
        Some(false),
        "{bad}"
    );
    assert!(!bad["problems"].as_array().unwrap().is_empty(), "{bad}");
    fs::write(&blob, &saved).unwrap();
    let good =
        structured(&session.call("nimvault_check", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        good.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "{good}"
    );

    fs::write(&secret_path, format!("{SECRET_TOKEN}\n{SECRET_EDIT}\n")).unwrap();
    let modified =
        structured(&session.call("nimvault_status", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        modified["entries"][0]["state"].as_str(),
        Some("modified"),
        "{modified}"
    );

    let unsealed = session.call("nimvault_unseal", serde_json::json!({"repo_path": &repo_s}));
    let unsealed_blob = unsealed.to_string();
    assert_clean(&unsealed_blob);
    assert!(unsealed.get("error").is_none(), "unseal: {unsealed}");
    assert_eq!(fs::read_to_string(&secret_path).unwrap(), body);

    let again =
        structured(&session.call("nimvault_status", serde_json::json!({"repo_path": &repo_s})));
    assert_eq!(
        again["entries"][0]["state"].as_str(),
        Some("in-sync"),
        "{again}"
    );

    let err = session.stderr.lock().unwrap().clone();
    assert_clean(&session.transcript);
    assert_clean(&err);
    if audit.is_file() {
        assert_clean(&fs::read_to_string(&audit).unwrap());
    }
}
