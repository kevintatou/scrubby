use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    webhook_secret: String,
    private_key_b64: String,
    out_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    let webhook_secret = env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_else(|_| {
        eprintln!("Missing STRIPE_WEBHOOK_SECRET");
        std::process::exit(1);
    });
    let private_key_b64 = env::var("SCRUBBY_PRIVATE_KEY_B64").unwrap_or_else(|_| {
        eprintln!("Missing SCRUBBY_PRIVATE_KEY_B64");
        std::process::exit(1);
    });
    let out_dir = env::var("SCRUBBY_LICENSE_OUT_DIR").unwrap_or_else(|_| "./licenses".to_string());

    let state = AppState {
        webhook_secret,
        private_key_b64,
        out_dir: PathBuf::from(out_dir),
    };

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    println!("Listening on http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

pub async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let sig = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, "Missing signature").into_response(),
    };

    if !verify_stripe_signature(&state.webhook_secret, &body, &sig) {
        return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
    }

    let payload: Value = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid JSON").into_response(),
    };

    let event_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if event_type != "checkout.session.completed" {
        return (StatusCode::OK, "Ignored event").into_response();
    }

    let data_obj = payload
        .get("data")
        .and_then(|v| v.get("object"))
        .cloned()
        .unwrap_or(Value::Null);

    let email = data_obj
        .get("customer_details")
        .and_then(|v| v.get("email"))
        .and_then(|v| v.as_str())
        .or_else(|| data_obj.get("customer_email").and_then(|v| v.as_str()))
        .unwrap_or("unknown");

    let device_id = data_obj
        .get("metadata")
        .and_then(|v| v.get("device_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if device_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing device_id").into_response();
    }

    let license = match build_license(&state.private_key_b64, email, device_id) {
        Ok(l) => l,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "License error").into_response(),
    };

    if let Err(_) = write_license(&state.out_dir, email, &license) {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Write error").into_response();
    }

    (StatusCode::OK, "OK").into_response()
}

fn verify_stripe_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let mut timestamp: Option<&str> = None;
    let mut v1: Option<&str> = None;
    for part in signature.split(',') {
        let mut kv = part.splitn(2, '=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().unwrap_or("");
        if k == "t" {
            timestamp = Some(v);
        } else if k == "v1" {
            v1 = Some(v);
        }
    }

    let (t, v1) = match (timestamp, v1) {
        (Some(t), Some(v1)) => (t, v1),
        _ => return false,
    };

    let signed_payload = format!("{}.{}", t, String::from_utf8_lossy(body));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let expected = mac.finalize().into_bytes();

    let provided = match hex::decode(v1) {
        Ok(b) => b,
        Err(_) => return false,
    };

    constant_time_eq(&expected, &provided)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut r = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        r |= x ^ y;
    }
    r == 0
}

fn build_license(private_key_b64: &str, email: &str, device_id: &str) -> Result<String, ()> {
    let priv_bytes = B64.decode(private_key_b64.as_bytes()).map_err(|_| ())?;
    if priv_bytes.len() != 32 {
        return Err(());
    }
    let signing = SigningKey::from_bytes(&priv_bytes[..32].try_into().unwrap());

    let mut payload = String::new();
    payload.push_str(&format!("email={}\n", email));
    payload.push_str("plan=pro\n");
    payload.push_str(&format!("device_id={}\n", device_id));

    let signature = signing.sign(payload.as_bytes());
    let license = format!(
        "SCRUBBY-LICENSE-1\npayload:{}\nsignature:{}\n",
        B64.encode(payload.as_bytes()),
        B64.encode(signature.to_bytes())
    );
    Ok(license)
}

fn write_license(out_dir: &PathBuf, email: &str, license: &str) -> Result<(), ()> {
    std::fs::create_dir_all(out_dir).map_err(|_| ())?;
    let safe = email.replace('@', "_").replace('.', "_");
    let mut path = out_dir.clone();
    path.push(format!("{}_license.key", safe));
    std::fs::write(path, license).map_err(|_| ())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_signature() {
        let secret = "whsec_test";
        let body = b"{\"test\":true}";
        let t = "12345";
        let signed_payload = format!("{}.{}", t, String::from_utf8_lossy(body));
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("t={},v1={}", t, sig);
        assert!(verify_stripe_signature(secret, body, &header));
    }
}
