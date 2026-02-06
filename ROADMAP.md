# Roadmap

Scrubby keeps scope intentionally tight. Every release focuses on a small set of user-facing improvements.

## v0.1 (current)
- Clipboard sanitize command
- Email, IP, UUID, JWT, and high-entropy token detection
- Clear placeholders and summary output

## v0.2 (Pro)
- Stable placeholders (e.g., <EMAIL_1>)
- JSON report output
- Config file for custom rules
- File and stdin mode

## v0.3
- Watch mode (optional, experimental)
- Rule packs (opt-in patterns for API keys)
- Better token heuristics
- Performance tuning and richer reports

Scope control: Scrubby stays CLI-first and local-only. No background daemons, no SaaS, no centralized telemetry.
