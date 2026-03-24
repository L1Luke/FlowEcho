use flowecho_core::crypto::generate_handshake_keypair;
use flowecho_core::error::ErrorCode;
use flowecho_core::pairing::{
    PairAuthenticationRequest, PairChallengeRequest, PairState, PairingCoordinator,
};
use flowecho_core::protocol::TrustState;

const NOW_MS: u64 = 1_700_000_000_000;

#[test]
fn issue_challenge_sets_state_and_ttl() {
    let mut coordinator = PairingCoordinator::default();
    let challenge = coordinator
        .issue_challenge(
            PairChallengeRequest {
                local_device_id: "mac-mini".to_string(),
                local_alias: "Luke Mac".to_string(),
                peer_ip: "192.168.1.44".to_string(),
            },
            NOW_MS,
        )
        .expect("issue challenge");

    assert_eq!(challenge.state, PairState::ChallengeIssued);
    assert_eq!(challenge.peer_ip, "192.168.1.44");
    assert_eq!(challenge.expires_at_ms, NOW_MS + 60_000);
    assert_eq!(challenge.attempts_remaining, 5);
    assert_eq!(challenge.otp_code.len(), 6);
    assert!(challenge.otp_code.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn authenticate_then_trust_records_trusted_device() {
    let mut coordinator = PairingCoordinator::default();
    let remote = generate_handshake_keypair();
    let challenge = coordinator
        .issue_challenge(
            PairChallengeRequest {
                local_device_id: "mac-mini".to_string(),
                local_alias: "Luke Mac".to_string(),
                peer_ip: "192.168.1.55".to_string(),
            },
            NOW_MS,
        )
        .expect("issue challenge");

    let auth = coordinator
        .authenticate(
            PairAuthenticationRequest {
                challenge_id: challenge.challenge_id.clone(),
                peer_ip: challenge.peer_ip.clone(),
                remote_device_id: "iphone-15".to_string(),
                remote_alias: "Luke iPhone".to_string(),
                otp_code: challenge.otp_code.clone(),
                remote_public_key: hex_encode(remote.public_key),
            },
            NOW_MS + 1,
        )
        .expect("authenticate");

    assert_eq!(auth.state, PairState::Authenticated);
    assert!(auth.session_key_id.starts_with("session-"));
    assert_eq!(
        coordinator.session_state(&challenge.challenge_id),
        Some(PairState::Authenticated)
    );

    let trusted = coordinator
        .trust_authenticated(&challenge.challenge_id)
        .expect("trust device");

    assert_eq!(trusted.device_id, "iphone-15");
    assert_eq!(trusted.alias, "Luke iPhone");
    assert_eq!(trusted.trust_state, TrustState::Trusted);
    assert_eq!(trusted.session_key_id, auth.session_key_id);
    assert_eq!(
        coordinator.session_state(&challenge.challenge_id),
        Some(PairState::Trusted)
    );
    assert_eq!(
        coordinator
            .trusted_device("iphone-15")
            .expect("trusted device")
            .session_key_id,
        auth.session_key_id
    );
}

#[test]
fn expired_otp_returns_pair_otp_expired() {
    let mut coordinator = PairingCoordinator::default();
    let remote = generate_handshake_keypair();
    let challenge = coordinator
        .issue_challenge(
            PairChallengeRequest {
                local_device_id: "mac-mini".to_string(),
                local_alias: "Luke Mac".to_string(),
                peer_ip: "192.168.1.77".to_string(),
            },
            NOW_MS,
        )
        .expect("issue challenge");

    let err = coordinator
        .authenticate(
            PairAuthenticationRequest {
                challenge_id: challenge.challenge_id,
                peer_ip: challenge.peer_ip,
                remote_device_id: "iphone-15".to_string(),
                remote_alias: "Luke iPhone".to_string(),
                otp_code: challenge.otp_code,
                remote_public_key: hex_encode(remote.public_key),
            },
            NOW_MS + 60_001,
        )
        .expect_err("must fail");

    assert_eq!(err.code, ErrorCode::PairOtpExpired);
}

#[test]
fn invalid_otp_times_out_after_five_failures() {
    let mut coordinator = PairingCoordinator::default();
    let remote = generate_handshake_keypair();
    let challenge = coordinator
        .issue_challenge(
            PairChallengeRequest {
                local_device_id: "mac-mini".to_string(),
                local_alias: "Luke Mac".to_string(),
                peer_ip: "192.168.1.88".to_string(),
            },
            NOW_MS,
        )
        .expect("issue challenge");

    for attempt in 0..5 {
        let err = coordinator
            .authenticate(
                PairAuthenticationRequest {
                    challenge_id: challenge.challenge_id.clone(),
                    peer_ip: challenge.peer_ip.clone(),
                    remote_device_id: "iphone-15".to_string(),
                    remote_alias: "Luke iPhone".to_string(),
                    otp_code: "000000".to_string(),
                    remote_public_key: hex_encode(remote.public_key),
                },
                NOW_MS + attempt,
            )
            .expect_err("must fail");
        assert_eq!(err.code, ErrorCode::PairOtpInvalid);
    }

    let timeout = coordinator
        .authenticate(
            PairAuthenticationRequest {
                challenge_id: challenge.challenge_id,
                peer_ip: challenge.peer_ip,
                remote_device_id: "iphone-15".to_string(),
                remote_alias: "Luke iPhone".to_string(),
                otp_code: challenge.otp_code,
                remote_public_key: hex_encode(remote.public_key),
            },
            NOW_MS + 10,
        )
        .expect_err("must fail");

    assert_eq!(timeout.code, ErrorCode::PairTimeout);
}

#[test]
fn authenticated_challenge_rejects_replay() {
    let mut coordinator = PairingCoordinator::default();
    let remote = generate_handshake_keypair();
    let challenge = coordinator
        .issue_challenge(
            PairChallengeRequest {
                local_device_id: "mac-mini".to_string(),
                local_alias: "Luke Mac".to_string(),
                peer_ip: "192.168.1.99".to_string(),
            },
            NOW_MS,
        )
        .expect("issue challenge");

    let request = PairAuthenticationRequest {
        challenge_id: challenge.challenge_id.clone(),
        peer_ip: challenge.peer_ip.clone(),
        remote_device_id: "iphone-15".to_string(),
        remote_alias: "Luke iPhone".to_string(),
        otp_code: challenge.otp_code.clone(),
        remote_public_key: hex_encode(remote.public_key),
    };

    coordinator
        .authenticate(request.clone(), NOW_MS + 1)
        .expect("authenticate");
    coordinator
        .trust_authenticated(&challenge.challenge_id)
        .expect("trust");

    let replay = coordinator
        .authenticate(request, NOW_MS + 2)
        .expect_err("must fail");

    assert_eq!(replay.code, ErrorCode::PairOtpInvalid);
}

fn hex_encode(bytes: [u8; 32]) -> String {
    bytes.into_iter().map(|byte| format!("{byte:02x}")).collect()
}
