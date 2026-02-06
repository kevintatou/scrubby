use scrubby::{format_summary, scrub_text};

#[test]
fn scrub_text_replaces_placeholders_and_formats_summary() {
    let input = "Email a@b.com IP 10.0.0.1 UUID 123e4567-e89b-42d3-a456-556642440000 JWT aaaa.bbbb.cccc TOKEN AbCDeF0123456789AbCDeF0123456789";
    let (out, summary) = scrub_text(input);

    assert!(out.contains("<EMAIL>"));
    assert!(out.contains("<IP>"));
    assert!(out.contains("<UUID>"));
    assert!(out.contains("<JWT>"));
    assert!(out.contains("<TOKEN>"));

    assert_eq!(summary.emails, 1);
    assert_eq!(summary.ips, 1);
    assert_eq!(summary.uuids, 1);
    assert_eq!(summary.jwts, 1);
    assert_eq!(summary.tokens, 1);

    let formatted = format_summary(&summary);
    assert!(formatted.contains("Scrubby cleaned your clipboard:"));
    assert!(formatted.contains("- Emails: 1"));
    assert!(formatted.contains("- IPs: 1"));
    assert!(formatted.contains("- UUIDs: 1"));
    assert!(formatted.contains("- JWTs: 1"));
    assert!(formatted.contains("- Tokens: 1"));
    assert!(formatted.contains("Safe to paste."));
}
