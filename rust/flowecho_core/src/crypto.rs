use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::{ErrorCode, FlowError, FlowResult};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HandshakeKeypair {
    pub private_key: [u8; 32],
    pub public_key: [u8; 32],
}

pub fn generate_handshake_keypair() -> HandshakeKeypair {
    let private = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&private);
    HandshakeKeypair {
        private_key: private.to_bytes(),
        public_key: public.to_bytes(),
    }
}

pub fn derive_session_key(
    local_private_key: [u8; 32],
    remote_public_key: [u8; 32],
    context: &[u8],
) -> FlowResult<[u8; 32]> {
    let local_private = StaticSecret::from(local_private_key);
    let remote_public = PublicKey::from(remote_public_key);
    let shared = local_private.diffie_hellman(&remote_public);
    let hk = Hkdf::<Sha256>::new(Some(context), shared.as_bytes());
    let mut output = [0u8; 32];
    hk.expand(b"flowecho-session-key-v1", &mut output)
        .map_err(|_| FlowError::new(ErrorCode::HandshakeFailed, "HKDF expand failed"))?;
    Ok(output)
}

pub fn seal(session_key: [u8; 32], plaintext: &[u8]) -> FlowResult<(Vec<u8>, [u8; 12])> {
    let key = Key::from_slice(&session_key);
    let cipher = ChaCha20Poly1305::new(key);
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let nonce_ref = Nonce::from_slice(&nonce);
    let ciphertext = cipher
        .encrypt(nonce_ref, plaintext)
        .map_err(|_| FlowError::new(ErrorCode::EncryptionFailed, "AEAD seal failed"))?;
    Ok((ciphertext, nonce))
}

pub fn open(session_key: [u8; 32], ciphertext: &[u8], nonce: [u8; 12]) -> FlowResult<Vec<u8>> {
    let key = Key::from_slice(&session_key);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce_ref = Nonce::from_slice(&nonce);
    let plaintext = cipher
        .decrypt(nonce_ref, ciphertext)
        .map_err(|_| FlowError::new(ErrorCode::DecryptionFailed, "AEAD open failed"))?;
    Ok(plaintext)
}
