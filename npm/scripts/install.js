// Minimal postinstall: prefer PATH cargo install; download from GitHub Releases when present.
const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const zlib = require("zlib");

const VERSION = require("../package.json").version;
const REPO = "HaoZeke/nimvault-mcp";
const BIN_DIR = path.join(__dirname, "..", "bin");
const ext = process.platform === "win32" ? ".exe" : "";
const dest = path.join(BIN_DIR, `nimvault-mcp${ext}`);

const PLATFORM_MAP = {
  "darwin-x64": "nimvault-mcp-x86_64-apple-darwin",
  "darwin-arm64": "nimvault-mcp-aarch64-apple-darwin",
  "linux-x64": "nimvault-mcp-x86_64-unknown-linux-gnu",
  "linux-arm64": "nimvault-mcp-aarch64-unknown-linux-gnu",
  "win32-x64": "nimvault-mcp-x86_64-pc-windows-msvc",
};

function fetch(urlStr) {
  return new Promise((resolve, reject) => {
    https
      .get(urlStr, { headers: { "User-Agent": "nimvault-mcp-npm" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return fetch(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${urlStr}`));
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  fs.mkdirSync(BIN_DIR, { recursive: true });
  // Prefer copying from cargo install location if developer built locally
  try {
    const which = execSync("which nimvault-mcp", { encoding: "utf8" }).trim();
    if (which && fs.existsSync(which)) {
      fs.copyFileSync(which, dest);
      fs.chmodSync(dest, 0o755);
      console.log(`Installed nimvault-mcp from ${which}`);
      return;
    }
  } catch (_) {}

  const key = `${process.platform}-${process.arch}`;
  const assetBase = PLATFORM_MAP[key];
  if (!assetBase) {
    console.warn(
      `No prebuilt binary for ${key}. Build with: cargo install --path . && copy target/release/nimvault-mcp to npm/bin/`
    );
    return;
  }
  const archive = process.platform === "win32" ? `${assetBase}.zip` : `${assetBase}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archive}`;
  try {
    console.log(`Downloading ${url}...`);
    const buf = await fetch(url);
    if (process.platform === "win32") {
      fs.writeFileSync(dest + ".zip", buf);
      console.warn("Extract the zip manually into npm/bin/ on Windows for now.");
      return;
    }
    const tar = zlib.gunzipSync(buf);
    // Extremely small tar walker: find file named nimvault-mcp
    // Fallback: write entire tarball note
    fs.writeFileSync(dest + ".tar", tar);
    try {
      execSync(`tar -xOf ${JSON.stringify(dest + ".tar")} nimvault-mcp > ${JSON.stringify(dest)}`, {
        stdio: "inherit",
        shell: true,
      });
      fs.chmodSync(dest, 0o755);
      fs.unlinkSync(dest + ".tar");
      console.log("Installed nimvault-mcp from GitHub release.");
    } catch (e) {
      console.warn(
        "Could not extract release archive. Build from source: cargo install --git https://github.com/HaoZeke/nimvault-mcp"
      );
    }
  } catch (e) {
    console.warn(
      `Release download failed (${e.message}). Build from source: cargo install --path .`
    );
  }
}

main().catch((e) => {
  console.warn(String(e));
});
