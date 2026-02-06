use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Default, Clone)]
pub struct Detections {
    pub emails: Vec<(usize, usize)>,
    pub ips: Vec<(usize, usize)>,
    pub uuids: Vec<(usize, usize)>,
    pub jwts: Vec<(usize, usize)>,
    pub tokens: Vec<(usize, usize)>,
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

pub fn detect(input: &str) -> Detections {
    let mut det = Detections::default();

    for m in EMAIL_RE.find_iter(input) {
        det.emails.push((m.start(), m.end()));
    }
    for m in IPV4_RE.find_iter(input) {
        det.ips.push((m.start(), m.end()));
    }
    for m in UUID_V4_RE.find_iter(input) {
        det.uuids.push((m.start(), m.end()));
    }
    for m in JWT_RE.find_iter(input) {
        det.jwts.push((m.start(), m.end()));
    }

    for m in TOKEN_CANDIDATE_RE.find_iter(input) {
        let s = &input[m.start()..m.end()];
        if shannon_entropy(s) >= 3.5 {
            det.tokens.push((m.start(), m.end()));
        }
    }

    det
}

pub fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for b in s.bytes() {
        counts[b as usize] += 1;
    }
    let len = s.len() as f64;
    let mut entropy = 0.0;
    for &c in counts.iter() {
        if c == 0 {
            continue;
        }
        let p = c as f64 / len;
        entropy -= p * p.log2();
    }
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_email() {
        let d = detect("email me at a.b+test@example.com");
        assert_eq!(d.emails.len(), 1);
    }

    #[test]
    fn detects_ipv4() {
        let d = detect("server 192.168.0.1 up");
        assert_eq!(d.ips.len(), 1);
    }

    #[test]
    fn detects_uuid_v4() {
        let d = detect("id 123e4567-e89b-42d3-a456-556642440000");
        assert_eq!(d.uuids.len(), 1);
    }

    #[test]
    fn detects_jwt() {
        let d = detect("token aaaa.bbbb.cccc");
        assert_eq!(d.jwts.len(), 1);
    }

    #[test]
    fn detects_high_entropy_token() {
        let token = "AbCDeF0123456789AbCDeF0123456789";
        let d = detect(token);
        assert_eq!(d.tokens.len(), 1);
    }

    #[test]
    fn entropy_low_for_repetitive() {
        let s = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert!(shannon_entropy(s) < 1.0);
    }
}
