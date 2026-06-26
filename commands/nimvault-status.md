---
description: Run nimvault status on a vault repo (defaults to NIMVAULT_DEFAULT_REPO or CWD)
---

Use the **nimvault_status** MCP tool.

- If the user named a path, pass it as `repo_path`.
- Else if `NIMVAULT_DEFAULT_REPO` is set in the environment, use that.
- Else use the current workspace git root if it contains `.vault/`.
- Summarize in-sync vs modified vs missing; suggest `nimvault seal` if status is slow/legacy.
