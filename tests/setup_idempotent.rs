//! `nimvault-mcp setup` writes client config once.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    common::mcp_bin()
}

fn temp_home(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("nv_mcp_setup_cli_{}_{}", std::process::id(), label));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn run(
    home: &std::path::Path,
    args: &[&str],
    path: &std::path::Path,
    marker: &std::path::Path,
) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PATH", path)
        .env("NIMVAULT_TEST_NPX_MARKER", marker)
        .output()
        .expect("spawn setup")
}

#[test]
fn setup_is_idempotent_and_skips_add_mcp_when_present() {
    let home = temp_home("home");
    let path_dir = temp_home("path");
    let marker = path_dir.join("npx-ran");
    let npx = path_dir.join("npx");
    fs::write(
        &npx,
        "#!/bin/sh\nprintf '%s\\n' ran > \"$NIMVAULT_TEST_NPX_MARKER\"\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&npx).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&npx, perms).unwrap();
    }

    let first = run(&home, &["setup", "--client=claude"], &path_dir, &marker);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(!marker.exists(), "claude setup must not call npx");
    let claude = fs::read(home.join(".claude.json")).unwrap();
    let second = run(&home, &["setup", "--client=claude"], &path_dir, &marker);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(claude, fs::read(home.join(".claude.json")).unwrap());
    assert!(!marker.exists());

    let v: serde_json::Value = serde_json::from_slice(&claude).unwrap();
    assert!(v.pointer("/mcpServers/nimvault/command").is_some());
    assert!(v.pointer("/mcpServers/nimvault2").is_none());

    let codex1 = run(&home, &["setup", "--client=codex"], &path_dir, &marker);
    assert!(
        codex1.status.success(),
        "{}",
        String::from_utf8_lossy(&codex1.stderr)
    );
    let codex = fs::read(home.join(".codex/config.toml")).unwrap();
    let codex2 = run(&home, &["setup", "--client=codex"], &path_dir, &marker);
    assert!(codex2.status.success());
    assert_eq!(codex, fs::read(home.join(".codex/config.toml")).unwrap());
    let text = String::from_utf8(codex).unwrap();
    assert_eq!(text.matches("[mcp_servers.nimvault]").count(), 1);

    let add = run(&home, &["setup", "--add-mcp"], &path_dir, &marker);
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    assert!(
        !marker.exists(),
        "add-mcp must not run once both clients exist"
    );
    assert_eq!(fs::read(home.join(".claude.json")).unwrap(), claude);

    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&path_dir);
}

#[test]
fn add_mcp_keeps_an_existing_command() {
    let home = temp_home("keep-home");
    let path_dir = temp_home("keep-path");
    let marker = path_dir.join("npx-ran");
    let npx = path_dir.join("npx");
    fs::write(
        &npx,
        "#!/bin/sh\nprintf '%s\\n' ran > \"$NIMVAULT_TEST_NPX_MARKER\"\nif [ -n \"$HOME\" ] && [ -f \"$HOME/.claude.json\" ]; then\n  printf '%s\\n' '{\"mcpServers\":{\"nimvault\":{\"command\":\"from-npx\"}}}' > \"$HOME/.claude.json\"\nfi\nexit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&npx).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&npx, perms).unwrap();
    }
    let custom = "{\n  \"mcpServers\": {\n    \"nimvault\": {\n      \"command\": \"custom-bin\"\n    }\n  }\n}\n";
    fs::write(home.join(".claude.json"), custom).unwrap();
    let out = run(
        &home,
        &["setup", "--add-mcp", "--client=claude", "--client=codex"],
        &path_dir,
        &marker,
    );
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(marker.exists(), "npx runs while codex is absent");
    assert_eq!(
        fs::read_to_string(home.join(".claude.json")).unwrap(),
        custom
    );
    let codex = fs::read_to_string(home.join(".codex/config.toml")).unwrap();
    assert!(codex.contains("[mcp_servers.nimvault]"), "{codex}");
    assert!(!codex.contains("from-npx"), "{codex}");
    let _ = fs::remove_dir_all(&home);
    let _ = fs::remove_dir_all(&path_dir);
}
