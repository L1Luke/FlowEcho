use flowecho_core::crypto::{derive_session_key, generate_handshake_keypair, open, seal};
use flowecho_core::error::ErrorCode;

#[test]
fn handshake_derives_same_session_key_for_both_peers() {
    let alice = generate_handshake_keypair();
    let bob = generate_handshake_keypair();
    let context = b"flowecho-phase-a";

    let alice_session =
        derive_session_key(alice.private_key, bob.public_key, context).expect("alice derive");
    let bob_session =
        derive_session_key(bob.private_key, alice.public_key, context).expect("bob derive");

    assert_eq!(alice_session, bob_session);
}

#[test]
fn aead_roundtrip_works_with_derived_session_key() {
    let alice = generate_handshake_keypair();
    let bob = generate_handshake_keypair();
    let context = b"flowecho-phase-a";
    let session_key =
        derive_session_key(alice.private_key, bob.public_key, context).expect("derive");

    let plaintext = b"flowecho clipboard payload";
    let (ciphertext, nonce) = seal(session_key, plaintext).expect("seal");
    let decrypted = open(session_key, &ciphertext, nonce).expect("open");

    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_with_wrong_key_fails_with_standard_error_code() {
    let alice = generate_handshake_keypair();
    let bob = generate_handshake_keypair();
    let charlie = generate_handshake_keypair();
    let context = b"flowecho-phase-a";

    let session_ab =
        derive_session_key(alice.private_key, bob.public_key, context).expect("derive AB");
    let session_ac =
        derive_session_key(alice.private_key, charlie.public_key, context).expect("derive AC");

    let (ciphertext, nonce) = seal(session_ab, b"secret").expect("seal");
    let err = open(session_ac, &ciphertext, nonce).expect_err("must fail to decrypt");
    assert_eq!(err.code, ErrorCode::DecryptionFailed);
}
