use std::path::PathBuf;

/// Built `nimvault-mcp` binary, next to this test executable's `deps` directory.
pub fn mcp_bin() -> PathBuf {
    let mut exe = std::env::current_exe().expect("current_exe");
    exe.pop();
    if exe.file_name().and_then(|s| s.to_str()) == Some("deps") {
        exe.pop();
    }
    exe.push(format!("nimvault-mcp{}", std::env::consts::EXE_SUFFIX));
    exe
}
