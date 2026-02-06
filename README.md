# Scrubby

Scrubby is a local-first, clipboard-focused CLI tool that sanitizes text before pasting it into AI tools (ChatGPT, Claude, Cursor, etc.). It runs entirely on your machine and never sends data anywhere.

## What it is
- A fast CLI that cleans clipboard content and puts it back, safe to paste.
- A simple, offline tool with clear placeholders.

## What it is NOT
- Not a SaaS.
- Not a background daemon.
- Not a cloud service or proxy.
 - Not an automatic “Ctrl+C hook” tool.

## Installation

From source:
```bash
cargo build --release
```

Then run:
```bash
./target/release/scrubby --clipboard
```

## Deploy Webhook (Render)
One‑click deploy for automated license delivery:

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/kevintatou/scrubby)

After deploy, set:
- `STRIPE_WEBHOOK_SECRET`
- `SCRUBBY_PRIVATE_KEY_B64`
- `SCRUBBY_LICENSE_OUT_DIR=/tmp/licenses`

Install script (coming soon):
```
curl -fsSL https://example.com/scrubby/install.sh | sh
```

For now, local install helper:
```
./scripts/install-local.sh
```

Distribution (today):
- Build a release binary: `cargo build --release`
- Provide a checksum: `sha256sum target/release/scrubby`
- Share the binary directly or upload to GitHub Releases.

Getting paid (simple starting path):
- Use Stripe Checkout and a webhook to generate device-bound licenses.
- Users place `license.key` at `~/.config/scrubby/license.key`.
- Pro features are gated locally; no network calls required.

### Linux prerequisites
Scrubby uses the system clipboard via common Linux utilities. Install one of:
- Wayland: `wl-clipboard` (provides `wl-copy` / `wl-paste`)
- X11: `xclip` or `xsel`

## Usage

Primary usage:
```bash
scrubby --clipboard
```

Experimental watch mode (opt-in only):
```bash
scrubby --watch
```

Pro-only modes (require Pro build + license):
```bash
scrubby --stdin
scrubby --file ./notes.txt
scrubby --json
scrubby --stable
scrubby --config ./scrubby.conf
```

Build with Pro features enabled:
```bash
cargo build --release --features pro-stable-placeholders,pro-json-report,pro-config,pro-file-stdin
```

Example config file:
```
stable_placeholders=true
json_report=false
interval_ms=500
```

Pro license file location:
- `~/.config/scrubby/license.key`
 - For local debug builds only: `SCRUBBY_LICENSE=DEV`

Device binding (optional):
- Run `scrubby --device-id` to get a device id.
- Licenses can be generated for a specific device id.

Check status:
```
systemctl --user status scrubby-watch.service
```

Stop it:
```
systemctl --user disable --now scrubby-watch.service
```

Example output:
```
Scrubby cleaned your clipboard:
- Emails: 1
- IPs: 1
- UUIDs: 1
- JWTs: 1
- Tokens: 2
Safe to paste.
```

## Hotkey examples

macOS (Shortcuts or Raycast):
- Create a Shortcut that runs `scrubby --clipboard`.
- Assign a hotkey to the Shortcut.

Linux (GNOME custom shortcut):
- Settings -> Keyboard -> Custom Shortcuts
- Command: `scrubby --clipboard`

Linux hotkey script example:
```bash
#!/usr/bin/env bash
set -euo pipefail
scrubby --clipboard
```

## Privacy
Scrubby has no telemetry and no network access. Everything happens locally.
