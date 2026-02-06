use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const DEFAULT_PUBLIC_KEY_B64: &str = "";

#[derive(Debug)]
pub struct LicenseError {
    pub message: String,
}

impl std::fmt::Display for LicenseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LicenseError {}

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub email: Option<String>,
    pub plan: Option<String>,
    pub expires: Option<String>,
    pub device_id: Option<String>,
}

pub fn check_license() -> Result<Option<LicenseInfo>, LicenseError> {
    if cfg!(debug_assertions) {
        if let Ok(v) = std::env::var("SCRUBBY_LICENSE") {
            if v.trim() == "DEV" {
                return Ok(Some(LicenseInfo {
                    email: Some("DEV".to_string()),
                    plan: Some("dev".to_string()),
                    expires: None,
                    device_id: None,
                }));
            }
        }
    }

    let mut path = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(p) => PathBuf::from(p),
        None => {
            let mut p = PathBuf::new();
            if let Some(home) = std::env::var_os("HOME") {
                p.push(home);
                p.push(".config");
            }
            p
        }
    };

    if path.as_os_str().is_empty() {
        return Ok(None);
    }

    path.push("scrubby");
    path.push("license.key");

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let public_key_b64 = option_env!("SCRUBBY_PUBLIC_KEY_B64").unwrap_or(DEFAULT_PUBLIC_KEY_B64);
    if public_key_b64.is_empty() {
        return Err(LicenseError {
            message: "Public key not configured. Build with SCRUBBY_PUBLIC_KEY_B64.".to_string(),
        });
    }

    let public_key_bytes = B64
        .decode(public_key_b64.as_bytes())
        .map_err(|_| LicenseError {
            message: "Invalid public key encoding".to_string(),
        })?;
    if public_key_bytes.len() != 32 {
        return Err(LicenseError {
            message: "Invalid public key length".to_string(),
        });
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&public_key_bytes[..32]);

    let info = verify_license_file_with_key(&content, &pk)?;
    enforce_device_binding(&info)?;
    Ok(Some(info))
}

fn verify_license_file_with_key(
    input: &str,
    public_key: &[u8; 32],
) -> Result<LicenseInfo, LicenseError> {
    let mut lines = input.lines().map(|l| l.trim());
    let header = lines.next().ok_or_else(|| LicenseError {
        message: "Invalid license file (missing header)".to_string(),
    })?;
    if header != "SCRUBBY-LICENSE-1" {
        return Err(LicenseError {
            message: "Invalid license file header".to_string(),
        });
    }

    let payload_line = lines.next().ok_or_else(|| LicenseError {
        message: "Invalid license file (missing payload)".to_string(),
    })?;
    let sig_line = lines.next().ok_or_else(|| LicenseError {
        message: "Invalid license file (missing signature)".to_string(),
    })?;

    let payload_b64 = payload_line
        .strip_prefix("payload:")
        .ok_or_else(|| LicenseError {
            message: "Invalid license file (payload prefix)".to_string(),
        })?;
    let sig_b64 = sig_line
        .strip_prefix("signature:")
        .ok_or_else(|| LicenseError {
            message: "Invalid license file (signature prefix)".to_string(),
        })?;

    let payload = B64
        .decode(payload_b64.as_bytes())
        .map_err(|_| LicenseError {
            message: "Invalid license payload encoding".to_string(),
        })?;
    let sig_bytes = B64.decode(sig_b64.as_bytes()).map_err(|_| LicenseError {
        message: "Invalid license signature encoding".to_string(),
    })?;

    let sig = Signature::from_slice(&sig_bytes).map_err(|_| LicenseError {
        message: "Invalid license signature".to_string(),
    })?;

    let pubkey = VerifyingKey::from_bytes(public_key).map_err(|_| LicenseError {
        message: "Invalid public key".to_string(),
    })?;

    pubkey.verify(&payload, &sig).map_err(|_| LicenseError {
        message: "License signature check failed".to_string(),
    })?;

    let payload_str = String::from_utf8(payload).map_err(|_| LicenseError {
        message: "Invalid license payload utf8".to_string(),
    })?;

    Ok(parse_payload(&payload_str))
}

fn parse_payload(payload: &str) -> LicenseInfo {
    let mut info = LicenseInfo {
        email: None,
        plan: None,
        expires: None,
        device_id: None,
    };
    for line in payload.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if value.is_empty() {
            continue;
        }
        match key {
            "email" => info.email = Some(value.to_string()),
            "plan" => info.plan = Some(value.to_string()),
            "expires" => info.expires = Some(value.to_string()),
            "device_id" => info.device_id = Some(value.to_string()),
            _ => {}
        }
    }
    info
}

fn enforce_device_binding(info: &LicenseInfo) -> Result<(), LicenseError> {
    if let Some(bound) = info.device_id.as_ref() {
        let current = current_device_id()?;
        if bound != &current {
            return Err(LicenseError {
                message: "License is not valid for this device".to_string(),
            });
        }
    }
    Ok(())
}

pub fn current_device_id() -> Result<String, LicenseError> {
    let machine_id = read_first_existing(&["/etc/machine-id", "/var/lib/dbus/machine-id"]);
    let hostname = read_first_existing(&["/etc/hostname"])
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "unknown-host".to_string());
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".to_string());

    let raw = format!(
        "{}|{}|{}",
        machine_id.unwrap_or_else(|| "no-machine-id".to_string()),
        hostname.trim(),
        user.trim()
    );
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    Ok(hex_lower(&digest))
}

fn read_first_existing(paths: &[&str]) -> Option<String> {
    for p in paths {
        if let Ok(s) = fs::read_to_string(p) {
            let v = s.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn dev_license_only_in_debug() {
        std::env::set_var("SCRUBBY_LICENSE", "DEV");
        let ok = check_license().unwrap();
        if cfg!(debug_assertions) {
            assert!(ok);
        } else {
            assert!(ok.is_none());
        }
        std::env::remove_var("SCRUBBY_LICENSE");
    }

    #[test]
    fn license_file_unblocks() {
        let dir = tempfile::tempdir().unwrap();
        let signing = SigningKey::generate(&mut OsRng);
        let verifying = signing.verifying_key();
        let payload = "email=test@example.com\nplan=pro\n";
        let signature = signing.sign(payload.as_bytes());
        let license = format!(
            "SCRUBBY-LICENSE-1\npayload:{}\nsignature:{}\n",
            B64.encode(payload.as_bytes()),
            B64.encode(signature.to_bytes())
        );

        let mut path = PathBuf::from(dir.path());
        path.push("scrubby");
        fs::create_dir_all(&path).unwrap();
        path.push("license.key");
        fs::write(&path, license).unwrap();

        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let content = fs::read_to_string(&path).unwrap();
        let info = verify_license_file_with_key(&content, verifying.as_bytes()).unwrap();
        assert_eq!(info.email, Some("test@example.com".to_string()));
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn missing_license_is_false() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        let ok = check_license().unwrap();
        assert!(ok.is_none());
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn device_binding_rejects_mismatch() {
        let info = LicenseInfo {
            email: None,
            plan: None,
            expires: None,
            device_id: Some("not-this-device".to_string()),
        };
        let err = enforce_device_binding(&info).unwrap_err();
        assert!(err.message.contains("device"));
    }
}
