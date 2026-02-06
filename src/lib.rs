pub mod clipboard;
pub mod config;
pub mod detectors;
pub mod license;
pub mod redactor;

use detectors::Detections;
use redactor::RedactionResult;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    pub emails: usize,
    pub ips: usize,
    pub uuids: usize,
    pub jwts: usize,
    pub tokens: usize,
}

impl Summary {
    pub fn total(&self) -> usize {
        self.emails + self.ips + self.uuids + self.jwts + self.tokens
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScrubOptions {
    pub stable_placeholders: bool,
}

pub fn scrub_text(input: &str) -> (String, Summary) {
    scrub_text_with_options(input, ScrubOptions::default())
}

pub fn scrub_text_with_options(input: &str, options: ScrubOptions) -> (String, Summary) {
    let detections: Detections = detectors::detect(input);
    let redacted: RedactionResult = redactor::redact(input, &detections, options);

    let summary = Summary {
        emails: redacted.counts.emails,
        ips: redacted.counts.ips,
        uuids: redacted.counts.uuids,
        jwts: redacted.counts.jwts,
        tokens: redacted.counts.tokens,
    };

    (redacted.text, summary)
}

pub fn format_summary(summary: &Summary) -> String {
    let mut lines = Vec::new();
    lines.push("Scrubby cleaned your clipboard:".to_string());
    lines.push(format!("- Emails: {}", summary.emails));
    lines.push(format!("- IPs: {}", summary.ips));
    lines.push(format!("- UUIDs: {}", summary.uuids));
    lines.push(format!("- JWTs: {}", summary.jwts));
    lines.push(format!("- Tokens: {}", summary.tokens));
    lines.push("Safe to paste.".to_string());
    lines.join("\n")
}
