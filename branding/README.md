# nimvault branding

## Canonical mark (do not invent alternatives)

**Source of truth:** [`logo/nimvault.svg`](logo/nimvault.svg)

That is the logo on [nimvault.rgoswami.me](https://nimvault.rgoswami.me) (`_static/nimvault.svg` in the Sphinx build — copy of this file). Teal vault gradient + **official Nim crown** + gold lock.

| File | Role |
|------|------|
| `logo/nimvault.svg` | **Canonical** — site header, OG image path, favicon source |
| `logo.svg` | Symlink-style copy of the canonical SVG for shallow paths |
| `logo.png` | Raster export for GitHub avatars / social (generated from SVG) |
| `_archive_session/` | One-off generated marks — **not** official |

## Usage

- Docs / Shibuya: `docs` copies into `_static/nimvault.svg` (see site deploy).
- MCP plugin README: point at `branding/logo/nimvault.svg` or `branding/logo.svg`.
- GitHub: set repo avatar from `logo.png` or upload the SVG-rendered PNG.

## Colors (from the mark)

| | |
|--|--|
| Vault teal | `#004D40` → `#00796B` |
| Door | `#00695C` / `#80CBC4` / `#E0F2F1` |
| Lock gold | `#FFD54F` / `#F9A825` |
| Nim crown | `#f3d400` / `#ffe953` |
