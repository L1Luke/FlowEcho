use std::collections::HashMap;

use rand_core::{OsRng, RngCore};

use crate::error::{ErrorCode, FlowError, FlowResult};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PairState {
    Init,
    ChallengeIssued,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PairChallengeRequest {
    pub local_device_id: String,
    pub local_alias: String,
    pub peer_ip: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PairChallenge {
    pub challenge_id: String,
    pub peer_ip: String,
    pub otp_code: String,
    pub expires_at_ms: u64,
    pub attempts_remaining: u8,
    pub state: PairState,
}

#[derive(Debug, Default)]
pub struct PairingCoordinator {
    next_challenge_id: u64,
    challenges: HashMap<String, PairChallenge>,
}

impl PairingCoordinator {
    pub fn issue_challenge(
        &mut self,
        req: PairChallengeRequest,
        now_ms: u64,
    ) -> FlowResult<PairChallenge> {
        if req.local_device_id.trim().is_empty()
            || req.local_alias.trim().is_empty()
            || req.peer_ip.trim().is_empty()
        {
            return Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "device identity and peer_ip are required",
            ));
        }

        self.next_challenge_id += 1;
        let challenge = PairChallenge {
            challenge_id: format!("pair-{}", self.next_challenge_id),
            peer_ip: req.peer_ip,
            otp_code: generate_otp_code(),
            expires_at_ms: now_ms + 60_000,
            attempts_remaining: 5,
            state: PairState::ChallengeIssued,
        };
        self.challenges
            .insert(challenge.challenge_id.clone(), challenge.clone());
        Ok(challenge)
    }
}

fn generate_otp_code() -> String {
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    format!("{:06}", u32::from_be_bytes(bytes) % 1_000_000)
}
