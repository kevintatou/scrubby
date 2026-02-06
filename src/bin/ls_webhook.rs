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
use serde::Deserialize;
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

#[derive(Deserialize)]
struct WebhookPayload {
    meta: Meta,
    data: Data,
}

#[derive(Deserialize)]
struct Meta {
    event_name: String,
    custom_data: Option<CustomData>,
}

#[derive(Deserialize)]
struct CustomData {
    device_id: Option<String>,
}

#[derive(Deserialize)]
struct Data {
    attributes: Attributes,
}

#[derive(Deserialize)]
struct Attributes {
    user_email: Option<String>,
}

#[tokio::main]
async fn main() {
    let webhook_secret = env::var("LEMON_WEBHOOK_SECRET").unwrap_or_else(|_| {
        eprintln!("Missing LEMON_WEBHOOK_SECRET");
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

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
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
    let sig = match headers.get("x-signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, "Missing signature").into_response(),
    };

    if !verify_signature(&state.webhook_secret, &body, &sig) {
        return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
    }

    let payload: WebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid JSON").into_response(),
    };

    if payload.meta.event_name != "order_created" {
        return (StatusCode::OK, "Ignored event").into_response();
    }

    let email = payload
        .data
        .attributes
        .user_email
        .unwrap_or_else(|| "unknown".to_string());
    let device_id = payload
        .meta
        .custom_data
        .and_then(|c| c.device_id)
        .unwrap_or_else(|| "".to_string());
    if device_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing device_id").into_response();
    }

    let license = match build_license(&state.private_key_b64, &email, &device_id) {
        Ok(l) => l,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "License error").into_response(),
    };

    if let Err(_) = write_license(&state.out_dir, &email, &license) {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Write error").into_response();
    }

    (StatusCode::OK, "OK").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum::http::HeaderMap;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    use ed25519_dalek::SigningKey;
    use hmac::{Hmac, Mac};
    use rand_core::OsRng;
    use sha2::Sha256;
    use std::fs;
    use tempfile::tempdir;

    fn sign_body(secret: &str, body: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let tag = mac.finalize().into_bytes();
        hex::encode(tag)
    }

    #[tokio::test]
    async fn end_to_end_webhook_writes_license() {
        let secret = "test_secret";
        let signing = SigningKey::generate(&mut OsRng);
        let priv_b64 = B64.encode(signing.to_bytes());
        let dir = tempdir().unwrap();

        let state = AppState {
            webhook_secret: secret.to_string(),
            private_key_b64: priv_b64,
            out_dir: dir.path().to_path_buf(),
        };

        let payload = r#"{
            "meta": {
                "event_name": "order_created",
                "custom_data": { "device_id": "device123" }
            },
            "data": {
                "attributes": { "user_email": "buyer@example.com" }
            }
        }"#;

        let body = payload.as_bytes();
        let sig = sign_body(secret, body);

        let mut headers = HeaderMap::new();
        headers.insert("x-signature", sig.parse().unwrap());

        let resp = handle_webhook(State(state), headers, Bytes::from(body))
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);

        let license_path = dir.path().join("buyer_example_com_license.key");
        let content = fs::read_to_string(&license_path).unwrap();
        assert!(content.contains("SCRUBBY-LICENSE-1"));
    }
}

fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let sig_bytes = match hex::decode(signature) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
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
    if !device_id.is_empty() {
        payload.push_str(&format!("device_id={}\n", device_id));
    }

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
