use flowecho_core::pairing::{PairChallengeRequest, PairState, PairingCoordinator};

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
