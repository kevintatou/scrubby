use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let mut email: Option<String> = None;
    let mut plan: Option<String> = None;
    let mut expires: Option<String> = None;
    let mut device_id: Option<String> = None;
    let mut out: Option<PathBuf> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--email" => email = args.next(),
            "--plan" => plan = args.next(),
            "--expires" => expires = args.next(),
            "--device-id" => device_id = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            _ => usage_and_exit(),
        }
    }

    let email = email.unwrap_or_else(|| usage_and_exit());
    let plan = plan.unwrap_or_else(|| "pro".to_string());
    let out = out.unwrap_or_else(|| PathBuf::from("license.key"));

    let priv_b64 = env::var("SCRUBBY_PRIVATE_KEY_B64").unwrap_or_else(|_| {
        eprintln!("Missing SCRUBBY_PRIVATE_KEY_B64");
        std::process::exit(1);
    });

    let priv_bytes = B64.decode(priv_b64.as_bytes()).unwrap_or_else(|_| {
        eprintln!("Invalid private key base64");
        std::process::exit(1);
    });

    if priv_bytes.len() != 32 {
        eprintln!("Invalid private key length");
        std::process::exit(1);
    }
    let signing = SigningKey::from_bytes(&priv_bytes[..32].try_into().unwrap());

    let mut payload = String::new();
    payload.push_str(&format!("email={}\n", email));
    payload.push_str(&format!("plan={}\n", plan));
    if let Some(e) = expires {
        payload.push_str(&format!("expires={}\n", e));
    }
    if let Some(d) = device_id {
        payload.push_str(&format!("device_id={}\n", d));
    }

    let signature: Signature = signing.sign(payload.as_bytes());

    let license = format!(
        "SCRUBBY-LICENSE-1\npayload:{}\nsignature:{}\n",
        B64.encode(payload.as_bytes()),
        B64.encode(signature.to_bytes())
    );

    fs::write(&out, license).unwrap_or_else(|e| {
        eprintln!("Failed to write license: {}", e);
        std::process::exit(1);
    });

    println!("Wrote {}", out.display());
}

fn usage_and_exit() -> ! {
    eprintln!(
        "Usage: license_sign --email <email> [--plan pro] [--expires YYYY-MM-DD] [--device-id <id>] --out <path>"
    );
    std::process::exit(1);
}
