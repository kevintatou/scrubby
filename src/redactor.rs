use once_cell::sync::Lazy;
use regex::Regex;

use crate::detectors::shannon_entropy;
use crate::detectors::Detections;
use crate::ScrubOptions;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RedactionCounts {
    pub emails: usize,
    pub ips: usize,
    pub uuids: usize,
    pub jwts: usize,
    pub tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionResult {
    pub text: String,
    pub counts: RedactionCounts,
}

static EMAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap());

static IPV4_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b").unwrap()
});

static UUID_V4_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}\b",
    )
    .unwrap()
});

static JWT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b").unwrap());

static TOKEN_CANDIDATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9_-]{32,}\b").unwrap());

pub fn redact(input: &str, _detections: &Detections, options: ScrubOptions) -> RedactionResult {
    let mut counts = RedactionCounts::default();
    let mut email_i = 0usize;
    let mut ip_i = 0usize;
    let mut uuid_i = 0usize;
    let mut jwt_i = 0usize;
    let mut token_i = 0usize;

    let (text, emails) = replace_with_count(
        input,
        &EMAIL_RE,
        "<EMAIL>",
        options.stable_placeholders,
        &mut email_i,
    );
    counts.emails += emails;

    let (text, ips) = replace_with_count(
        &text,
        &IPV4_RE,
        "<IP>",
        options.stable_placeholders,
        &mut ip_i,
    );
    counts.ips += ips;

    let (text, uuids) = replace_with_count(
        &text,
        &UUID_V4_RE,
        "<UUID>",
        options.stable_placeholders,
        &mut uuid_i,
    );
    counts.uuids += uuids;

    let (text, jwts) = replace_with_count(
        &text,
        &JWT_RE,
        "<JWT>",
        options.stable_placeholders,
        &mut jwt_i,
    );
    counts.jwts += jwts;

    let (text, tokens) = replace_tokens(&text, options.stable_placeholders, &mut token_i);
    counts.tokens += tokens;

    // TODO(pro-stable-placeholders): gate stable placeholders behind license checks.
    RedactionResult { text, counts }
}

fn replace_with_count(
    input: &str,
    re: &Regex,
    replacement: &str,
    stable: bool,
    counter: &mut usize,
) -> (String, usize) {
    let mut count = 0usize;
    let mut out = String::with_capacity(input.len());
    let mut last = 0usize;
    for m in re.find_iter(input) {
        out.push_str(&input[last..m.start()]);
        if stable {
            *counter += 1;
            out.push_str(&format!(
                "{}_{}",
                replacement.trim_end_matches('>'),
                *counter
            ));
            out.push('>');
        } else {
            out.push_str(replacement);
        }
        count += 1;
        last = m.end();
    }
    out.push_str(&input[last..]);
    (out, count)
}

fn replace_tokens(input: &str, stable: bool, counter: &mut usize) -> (String, usize) {
    let mut count = 0usize;
    let mut out = String::with_capacity(input.len());
    let mut last = 0usize;
    for m in TOKEN_CANDIDATE_RE.find_iter(input) {
        let s = &input[m.start()..m.end()];
        out.push_str(&input[last..m.start()]);
        if shannon_entropy(s) >= 3.5 {
            if stable {
                *counter += 1;
                out.push_str("<TOKEN_");
                out.push_str(&counter.to_string());
                out.push('>');
            } else {
                out.push_str("<TOKEN>");
            }
            count += 1;
        } else {
            out.push_str(s);
        }
        last = m.end();
    }
    out.push_str(&input[last..]);
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScrubOptions;

    #[test]
    fn stable_placeholders_increment() {
        let opts = ScrubOptions {
            stable_placeholders: true,
        };
        let det = Detections::default();
        let input = "a@b.com a@b.com";
        let redacted = redact(input, &det, opts);
        assert!(redacted.text.contains("<EMAIL_1>"));
        assert!(redacted.text.contains("<EMAIL_2>"));
    }
}
