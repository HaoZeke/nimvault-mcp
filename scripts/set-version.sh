#!/usr/bin/env bash
set -euo pipefail
version="$1"
VERSION="$version" perl -0pi -e 's/^version = "[^"]*"/version = "$ENV{VERSION}"/m' Cargo.toml
VERSION="$version" perl -0pi -e 's/"version": "[^"]*"/"version": "$ENV{VERSION}"/' npm/package.json plugin.json .claude-plugin/plugin.json 2>/dev/null || true
echo "version -> $version"
