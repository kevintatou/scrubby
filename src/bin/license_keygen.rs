use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

fn main() {
    let signing = SigningKey::generate(&mut OsRng);
    let verifying = signing.verifying_key();
    println!("PRIVATE_KEY_B64={}", B64.encode(signing.to_bytes()));
    println!("PUBLIC_KEY_B64={}", B64.encode(verifying.to_bytes()));
}
