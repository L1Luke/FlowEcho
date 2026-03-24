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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PairAcceptance {
    pub challenge_id: String,
    pub local_device_id: String,
    pub local_alias: String,
    pub local_public_key: String,
    pub remote_trust: DeviceTrust,
}

#[derive(Debug, Default)]
pub struct PairingCoordinator {
    next_challenge_id: u64,
    challenges: HashMap<String, PendingChallenge>,
    trusted_devices: HashMap<String, DeviceTrust>,
    trusted_endpoints: HashMap<String, TrustedPeerRecord>,
    session_keys_by_device: HashMap<String, [u8; 32]>,
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
            local_device_id: req.local_device_id,
            local_alias: req.local_alias,
            local_private_key: keypair.private_key,
            local_public_key: encode_hex_key(keypair.public_key),
            remote_device_id: None,
            remote_alias: None,
            remote_port: None,
            session_key_id: None,
            session_key: None,
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
        if pending.challenge.attempts_remaining == 0 {
            return Err(FlowError::new(ErrorCode::PairTimeout, "pairing challenge timed out"));
        }
        if pending.challenge.state != PairState::ChallengeIssued {
            return Err(FlowError::new(ErrorCode::PairOtpInvalid, "otp already consumed"));
        }
        if now_ms > pending.challenge.expires_at_ms {
            return Err(FlowError::new(ErrorCode::PairOtpExpired, "otp expired"));
        }
        if req.otp_code != pending.challenge.otp_code {
            pending.challenge.attempts_remaining -= 1;
            return Err(FlowError::new(ErrorCode::PairOtpInvalid, "otp invalid"));
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
        pending.session_key = Some(session_key);
        Ok(auth)
    }

    pub fn complete_network_pairing(
        &mut self,
        peer_ip: &str,
        peer_port: u16,
        remote_device_id: &str,
        remote_alias: &str,
        otp_code: &str,
        remote_public_key: &str,
        now_ms: u64,
    ) -> FlowResult<PairAcceptance> {
        let challenge_id = self
            .find_active_challenge_id(peer_ip)
            .ok_or_else(|| FlowError::new(ErrorCode::InvalidRequest, "challenge not found"))?;

        {
            let pending = self
                .challenges
                .get_mut(&challenge_id)
                .ok_or_else(|| FlowError::new(ErrorCode::InvalidRequest, "challenge not found"))?;
            pending.remote_port = Some(peer_port);
        }

        self.authenticate(
            PairAuthenticationRequest {
                challenge_id: challenge_id.clone(),
                peer_ip: peer_ip.to_string(),
                remote_device_id: remote_device_id.to_string(),
                remote_alias: remote_alias.to_string(),
                otp_code: otp_code.to_string(),
                remote_public_key: remote_public_key.to_string(),
            },
            now_ms,
        )?;

        let trust = self.trust_authenticated(&challenge_id)?;
        let pending = self
            .challenges
            .get(&challenge_id)
            .ok_or_else(|| FlowError::new(ErrorCode::Internal, "missing accepted challenge"))?;

        Ok(PairAcceptance {
            challenge_id,
            local_device_id: pending.local_device_id.clone(),
            local_alias: pending.local_alias.clone(),
            local_public_key: pending.local_public_key.clone(),
            remote_trust: trust,
        })
    }

    pub fn register_trusted_peer(
        &mut self,
        peer_ip: String,
        peer_port: u16,
        device_id: String,
        alias: String,
        session_key: [u8; 32],
        session_key_id: String,
    ) -> DeviceTrust {
        let trust = DeviceTrust {
            device_id: device_id.clone(),
            alias,
            trust_state: TrustState::Trusted,
            session_key_id,
        };
        self.trusted_devices
            .insert(device_id.clone(), trust.clone());
        self.session_keys_by_device.insert(device_id.clone(), session_key);
        self.trusted_endpoints.insert(
            endpoint_key(&peer_ip, peer_port),
            TrustedPeerRecord { trust: trust.clone() },
        );
        trust
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
        if let Some(session_key) = pending.session_key {
            self.session_keys_by_device
                .insert(trust.device_id.clone(), session_key);
        }
        if let Some(remote_port) = pending.remote_port {
            self.trusted_endpoints.insert(
                endpoint_key(&pending.challenge.peer_ip, remote_port),
                TrustedPeerRecord { trust: trust.clone() },
            );
        }
        Ok(trust)
    }

    pub fn session_state(&self, challenge_id: &str) -> Option<PairState> {
        self.challenges.get(challenge_id).map(|p| p.challenge.state)
    }

    pub fn trusted_device(&self, device_id: &str) -> Option<DeviceTrust> {
        self.trusted_devices.get(device_id).cloned()
    }

    pub fn trusted_device_for_endpoint(&self, peer_ip: &str, peer_port: u16) -> Option<DeviceTrust> {
        self.trusted_endpoints
            .get(&endpoint_key(peer_ip, peer_port))
            .map(|record| record.trust.clone())
    }

    pub fn session_key_for_device(&self, device_id: &str) -> Option<[u8; 32]> {
        self.session_keys_by_device.get(device_id).copied()
    }

    pub fn session_key_for_endpoint(&self, peer_ip: &str, peer_port: u16) -> Option<[u8; 32]> {
        self.trusted_endpoints
            .get(&endpoint_key(peer_ip, peer_port))
            .and_then(|record| self.session_keys_by_device.get(&record.trust.device_id).copied())
    }

    fn find_active_challenge_id(&self, peer_ip: &str) -> Option<String> {
        self.challenges
            .iter()
            .filter(|(_, pending)| {
                pending.challenge.peer_ip == peer_ip
                    && pending.challenge.state == PairState::ChallengeIssued
            })
            .max_by_key(|(_, pending)| pending.challenge.challenge_id.clone())
            .map(|(challenge_id, _)| challenge_id.clone())
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
    local_device_id: String,
    local_alias: String,
    local_private_key: [u8; 32],
    local_public_key: String,
    remote_device_id: Option<String>,
    remote_alias: Option<String>,
    remote_port: Option<u16>,
    session_key_id: Option<String>,
    session_key: Option<[u8; 32]>,
}

#[derive(Debug, Clone)]
struct TrustedPeerRecord {
    trust: DeviceTrust,
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

fn encode_hex_key(input: [u8; 32]) -> String {
    input.iter().map(|byte| format!("{byte:02x}")).collect()
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

fn endpoint_key(peer_ip: &str, peer_port: u16) -> String {
    format!("{peer_ip}:{peer_port}")
}
