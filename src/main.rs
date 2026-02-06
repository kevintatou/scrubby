use scrubby::clipboard::{read_clipboard, write_clipboard};
use scrubby::config::load_config;
use scrubby::license::{check_license, current_device_id, LicenseInfo};
use scrubby::{format_summary, scrub_text_with_options, ScrubOptions, Summary};
use std::io::{self, Read};
use std::path::PathBuf;

fn print_usage() {
    eprintln!("Usage: scrubby [--clipboard | --watch] [options]");
    eprintln!("\nOptions:");
    eprintln!("  --clipboard    Sanitize current clipboard text");
    eprintln!("  --watch        Watch clipboard and sanitize on change (experimental)");
    eprintln!("  --device-id    Print device id for license binding");
    eprintln!("  --interval-ms  Poll interval for --watch (default: 750)");
    eprintln!("  --stdin        Read from stdin and print sanitized text");
    eprintln!("  --file <path>  Read file and print sanitized text");
    eprintln!("  --json         Print JSON report instead of text summary");
    eprintln!("  --stable       Use stable placeholders (e.g., <EMAIL_1>)");
    eprintln!("  --config <path>  Load config file");
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut mode: Option<String> = None;
    let mut interval_ms: u64 = 750;
    let mut json = false;
    let mut stable = false;
    let mut config_path: Option<PathBuf> = None;
    let mut file_path: Option<PathBuf> = None;
    let mut stdin_mode = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--clipboard" | "--watch" => {
                if mode.is_some() {
                    print_usage();
                    std::process::exit(1);
                }
                mode = Some(arg);
            }
            "--interval-ms" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => {
                        print_usage();
                        std::process::exit(1);
                    }
                };
                interval_ms = match v.parse::<u64>() {
                    Ok(n) if n >= 100 => n,
                    _ => {
                        eprintln!("Scrubby error: --interval-ms must be >= 100");
                        std::process::exit(1);
                    }
                };
            }
            "--json" => {
                json = true;
            }
            "--stable" => {
                stable = true;
            }
            "--config" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => {
                        print_usage();
                        std::process::exit(1);
                    }
                };
                config_path = Some(PathBuf::from(v));
            }
            "--file" => {
                let v = match args.next() {
                    Some(v) => v,
                    None => {
                        print_usage();
                        std::process::exit(1);
                    }
                };
                file_path = Some(PathBuf::from(v));
            }
            "--stdin" => {
                stdin_mode = true;
            }
            "--device-id" => {
                let id = match current_device_id() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Scrubby error: {}", e);
                        std::process::exit(1);
                    }
                };
                println!("{}", id);
                return;
            }
            _ => {
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let mode = mode.unwrap_or_else(|| "--clipboard".to_string());

    if stdin_mode && file_path.is_some() {
        eprintln!("Scrubby error: --stdin and --file are mutually exclusive");
        std::process::exit(1);
    }

    if mode == "--watch" && (stdin_mode || file_path.is_some()) {
        eprintln!("Scrubby error: --watch cannot be used with --stdin or --file");
        std::process::exit(1);
    }

    let license = apply_feature_gates(
        json,
        stable,
        config_path.is_some(),
        stdin_mode || file_path.is_some(),
    );

    let mut options = ScrubOptions::default();
    if let Some(path) = config_path.as_ref() {
        match load_config(path) {
            Ok(cfg) => {
                if let Some(v) = cfg.stable_placeholders {
                    options.stable_placeholders = v;
                }
                if let Some(v) = cfg.json_report {
                    json = v;
                }
                if let Some(v) = cfg.interval_ms {
                    interval_ms = v;
                }
            }
            Err(e) => {
                eprintln!("Scrubby error: {}", e);
                std::process::exit(1);
            }
        }
    }

    if stable {
        options.stable_placeholders = true;
    }

    if let Some(info) = license.as_ref() {
        if let Some(email) = info.email.as_ref() {
            eprintln!("Scrubby Pro licensed to {}", email);
        } else {
            eprintln!("Scrubby Pro license verified");
        }
    }

    if stdin_mode {
        run_stdin(json, options);
        return;
    }
    if let Some(path) = file_path {
        run_file(&path, json, options);
        return;
    }

    if mode == "--clipboard" {
        run_once(json, options);
    } else {
        run_watch(interval_ms, json, options);
    }
}

fn run_once(json: bool, options: ScrubOptions) {
    let input = match read_clipboard() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Scrubby error: {}", e);
            std::process::exit(2);
        }
    };

    let (sanitized, summary) = scrub_text_with_options(&input, options);

    if let Err(e) = write_clipboard(&sanitized) {
        eprintln!("Scrubby error: {}", e);
        std::process::exit(3);
    }

    // TODO(pro-json-report): support --json output
    output_report(json, &summary, None);
}

fn run_watch(interval_ms: u64, json: bool, options: ScrubOptions) {
    let mut last_seen = String::new();
    let mut last_written = String::new();
    loop {
        let input = match read_clipboard() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Scrubby error: {}", e);
                std::process::exit(2);
            }
        };

        if input != last_seen {
            last_seen = input.clone();
            let (sanitized, summary) = scrub_text_with_options(&input, options);
            if sanitized != input && sanitized != last_written {
                if let Err(e) = write_clipboard(&sanitized) {
                    eprintln!("Scrubby error: {}", e);
                    std::process::exit(3);
                }
                last_written = sanitized;
                output_report(json, &summary, None);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
    }
}

fn run_stdin(json: bool, options: ScrubOptions) {
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Scrubby error: {}", e);
        std::process::exit(2);
    }
    let (sanitized, summary) = scrub_text_with_options(&input, options);
    println!("{}", sanitized);
    if json {
        eprintln!("{}", json_report(&summary));
    }
}

fn run_file(path: &PathBuf, json: bool, options: ScrubOptions) {
    let input = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Scrubby error: {}", e);
            std::process::exit(2);
        }
    };
    let (sanitized, summary) = scrub_text_with_options(&input, options);
    println!("{}", sanitized);
    if json {
        eprintln!("{}", json_report(&summary));
    }
}

fn output_report(json: bool, summary: &Summary, extra: Option<&str>) {
    if json {
        let mut out = json_report(summary);
        if let Some(e) = extra {
            out.push_str(e);
        }
        println!("{}", out);
    } else {
        println!("{}", format_summary(summary));
    }
}

fn json_report(summary: &Summary) -> String {
    format!(
        "{{\"emails\":{},\"ips\":{},\"uuids\":{},\"jwts\":{},\"tokens\":{},\"safe_to_paste\":true}}",
        summary.emails, summary.ips, summary.uuids, summary.jwts, summary.tokens
    )
}

fn apply_feature_gates(
    json: bool,
    stable: bool,
    config: bool,
    file_stdin: bool,
) -> Option<LicenseInfo> {
    if json {
        #[cfg(not(feature = "pro-json-report"))]
        {
            eprintln!(
                "Scrubby error: --json is a Pro feature (build with feature pro-json-report)"
            );
            std::process::exit(1);
        }
    }
    if stable {
        #[cfg(not(feature = "pro-stable-placeholders"))]
        {
            eprintln!("Scrubby error: --stable is a Pro feature (build with feature pro-stable-placeholders)");
            std::process::exit(1);
        }
    }
    if config {
        #[cfg(not(feature = "pro-config"))]
        {
            eprintln!("Scrubby error: --config is a Pro feature (build with feature pro-config)");
            std::process::exit(1);
        }
    }
    if file_stdin {
        #[cfg(not(feature = "pro-file-stdin"))]
        {
            eprintln!("Scrubby error: --file/--stdin is a Pro feature (build with feature pro-file-stdin)");
            std::process::exit(1);
        }
    }

    if json || stable || config || file_stdin {
        let license = match check_license() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Scrubby error: {}", e);
                std::process::exit(3);
            }
        };
        if license.is_none() {
            eprintln!("Scrubby error: Pro features require a license (set SCRUBBY_LICENSE=DEV in debug builds for local testing)");
            std::process::exit(3);
        }
        return license;
    }
    None
}
