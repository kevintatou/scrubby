use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::SigningKey;
use rand_core::{OsRng, RngCore};

fn main() {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let signing = SigningKey::from_bytes(&bytes);
    let verifying = signing.verifying_key();
    println!("PRIVATE_KEY_B64={}", B64.encode(signing.to_bytes()));
    println!("PUBLIC_KEY_B64={}", B64.encode(verifying.to_bytes()));
}
