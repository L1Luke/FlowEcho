use std::collections::HashMap;

use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::crypto::{derive_session_key, generate_handshake_keypair};
use crate::error::{ErrorCode, FlowError, FlowResult};
use crate::protocol::{DeviceTrust, TrustState};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PairState {
    Init,
    ChallengeIssued,
    Authenticated,
    Trusted,
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PairAuthenticationRequest {
    pub challenge_id: String,
    pub peer_ip: String,
    pub remote_device_id: String,
    pub remote_alias: String,
    pub otp_code: String,
    pub remote_public_key: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PairAuthentication {
    pub challenge_id: String,
    pub device_id: String,
    pub alias: String,
    pub session_key_id: String,
    pub state: PairState,
}

#[derive(Debug, Default)]
pub struct PairingCoordinator {
    next_challenge_id: u64,
    challenges: HashMap<String, PendingChallenge>,
    trusted_devices: HashMap<String, DeviceTrust>,
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
        let keypair = generate_handshake_keypair();
        let challenge = PairChallenge {
            challenge_id: format!("pair-{}", self.next_challenge_id),
            peer_ip: req.peer_ip,
            otp_code: generate_otp_code(),
            expires_at_ms: now_ms + 60_000,
            attempts_remaining: 5,
            state: PairState::ChallengeIssued,
        };
        let pending = PendingChallenge {
            challenge: challenge.clone(),
            local_private_key: keypair.private_key,
            remote_device_id: None,
            remote_alias: None,
            session_key_id: None,
        };
        self.challenges
            .insert(challenge.challenge_id.clone(), pending);
        Ok(challenge)
    }

    pub fn authenticate(
        &mut self,
        req: PairAuthenticationRequest,
        now_ms: u64,
    ) -> FlowResult<PairAuthentication> {
        let pending = self
            .challenges
            .get_mut(&req.challenge_id)
            .ok_or_else(|| FlowError::new(ErrorCode::InvalidRequest, "challenge not found"))?;
        if req.peer_ip != pending.challenge.peer_ip {
            return Err(FlowError::new(ErrorCode::InvalidRequest, "peer_ip mismatch"));
        }
        if now_ms > pending.challenge.expires_at_ms {
            return Err(FlowError::new(ErrorCode::PairOtpExpired, "otp expired"));
        }
        if req.otp_code != pending.challenge.otp_code {
            return Err(FlowError::new(ErrorCode::InvalidVerifyCode, "otp invalid"));
        }

        let session_key = derive_session_key(
            pending.local_private_key,
            decode_hex_key(&req.remote_public_key)?,
            pending.challenge.challenge_id.as_bytes(),
        )?;
        let auth = PairAuthentication {
            challenge_id: req.challenge_id,
            device_id: req.remote_device_id.clone(),
            alias: req.remote_alias.clone(),
            session_key_id: session_key_id(&session_key),
            state: PairState::Authenticated,
        };
        pending.challenge.state = PairState::Authenticated;
        pending.remote_device_id = Some(req.remote_device_id);
        pending.remote_alias = Some(req.remote_alias);
        pending.session_key_id = Some(auth.session_key_id.clone());
        Ok(auth)
    }

    pub fn trust_authenticated(&mut self, challenge_id: &str) -> FlowResult<DeviceTrust> {
        let pending = self
            .challenges
            .get_mut(challenge_id)
            .ok_or_else(|| FlowError::new(ErrorCode::InvalidRequest, "challenge not found"))?;
        if pending.challenge.state != PairState::Authenticated {
            return Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "challenge is not authenticated",
            ));
        }

        pending.challenge.state = PairState::Trusted;
        let trust = DeviceTrust {
            device_id: pending
                .remote_device_id
                .clone()
                .ok_or_else(|| FlowError::new(ErrorCode::Internal, "missing remote device"))?,
            alias: pending
                .remote_alias
                .clone()
                .ok_or_else(|| FlowError::new(ErrorCode::Internal, "missing remote alias"))?,
            trust_state: TrustState::Trusted,
            session_key_id: pending
                .session_key_id
                .clone()
                .ok_or_else(|| FlowError::new(ErrorCode::Internal, "missing session key id"))?,
        };
        self.trusted_devices
            .insert(trust.device_id.clone(), trust.clone());
        Ok(trust)
    }

    pub fn session_state(&self, challenge_id: &str) -> Option<PairState> {
        self.challenges.get(challenge_id).map(|p| p.challenge.state)
    }

    pub fn trusted_device(&self, device_id: &str) -> Option<DeviceTrust> {
        self.trusted_devices.get(device_id).cloned()
    }
}

fn generate_otp_code() -> String {
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    format!("{:06}", u32::from_be_bytes(bytes) % 1_000_000)
}

#[derive(Debug)]
struct PendingChallenge {
    challenge: PairChallenge,
    local_private_key: [u8; 32],
    remote_device_id: Option<String>,
    remote_alias: Option<String>,
    session_key_id: Option<String>,
}

fn decode_hex_key(input: &str) -> FlowResult<[u8; 32]> {
    if input.len() != 64 {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "remote_public_key must be 64 hex chars",
        ));
    }
    let mut output = [0u8; 32];
    for (index, slot) in output.iter_mut().enumerate() {
        let start = index * 2;
        *slot = u8::from_str_radix(&input[start..start + 2], 16)
            .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "remote_public_key is not valid hex"))?;
    }
    Ok(output)
}

fn session_key_id(session_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(session_key);
    format!(
        "session-{}",
        digest
            .iter()
            .take(12)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
