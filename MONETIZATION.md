# Monetization

## Free vs Pro
Free:
- Clipboard sanitize command
- Core detectors and placeholders
- Local-only execution

Pro:
- Stable placeholders (e.g., <EMAIL_1>)
- JSON report output
- Config file support
- File/stdin mode

## Pricing
- Individual: $19–$29 one-time
- Teams: per-seat offline license

## Offline license model
Scrubby uses signed offline license files. No login, no telemetry, no SaaS dependency.

Distribution and payment (simple path):
- Use Stripe Checkout + webhook.
- Deliver a per-user `license.key` file.
- User places it at `~/.config/scrubby/license.key`.

## Why no SaaS
Scrubby’s value is in local, private scrubbing. A cloud service would add risk and friction without adding core value.

## Target users
- Developers
- Security-conscious individuals
- Small teams who want to avoid accidental leaks into AI tools
