use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::crypto::{derive_session_key, generate_handshake_keypair};
use crate::error::{ErrorCode, FlowError, FlowResult};
use crate::pairing::{PairAcceptance, PairChallengeRequest, PairingCoordinator};
use crate::paste_router::decide_route;
use crate::protocol::{
    AppScope, ApplyPasteRequest, DeviceTrust, PairDeviceRequest, PairingChallenge,
    PastePolicy, PastePolicyMode, PasteResult, PasteSource, PublishClipboardRequest,
    ResumeTransferRequest, SendFileRequest, SendTextRequest, SetPastePolicyResponse,
    StartPairingRequest, StartTransferRequest, SyncAck, TransferOutcome,
    TransferSession, TransferState,
};
use crate::transfer::{
    build_resume_token, create_file_plan, recv_encrypted_packet, send_encrypted_packet,
    TransferAck, TransferCompleted, TransferCoordinator, TransferPacket,
};
use crate::transport::{transport_adapter, TransportEndpoint, TransportFrame, TransportMode, TransportSession};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:45123";
const PAIR_AUTH_FRAME_TYPE: u8 = 0x11;
const PAIR_AUTH_RESPONSE_FRAME_TYPE: u8 = 0x12;
const TRANSFER_HELLO_FRAME_TYPE: u8 = 0x20;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReceivedTextPayload {
    pub peer_ip: String,
    pub payload_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReceivedFilePayload {
    pub peer_ip: String,
    pub payload_id: String,
    pub payload_hash: String,
    pub file_path: PathBuf,
}

#[derive(Clone)]
pub struct FlowEchoService {
    inner: Arc<FlowEchoServiceInner>,
}

struct FlowEchoServiceInner {
    bind_addr: String,
    session_seq: AtomicU64,
    local_identity: RwLock<Option<LocalIdentity>>,
    pairing: RwLock<PairingCoordinator>,
    paste_policy: RwLock<PastePolicy>,
    inbound_transfers: Mutex<TransferCoordinator>,
    outbound_transfers: Mutex<HashMap<String, OutboundTransferRecord>>,
    received_texts: RwLock<Vec<ReceivedTextPayload>>,
    received_files: RwLock<Vec<ReceivedFilePayload>>,
    listener: Mutex<Option<ListenerRuntime>>,
}

#[derive(Debug, Clone)]
struct LocalIdentity {
    device_id: String,
}

#[derive(Debug, Clone)]
struct ListenerRuntime {
    endpoint: TransportEndpoint,
}

#[derive(Debug, Clone)]
struct OutboundTransferRecord {
    session_id: String,
    source_path: PathBuf,
    plan: crate::transfer::TransferPlan,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct PairingWireRequest {
    otp_code: String,
    remote_device_id: String,
    remote_alias: String,
    remote_public_key: String,
    callback_port: u16,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct PairingWireResponse {
    challenge_id: String,
    device_id: String,
    alias: String,
    local_public_key: String,
    session_key_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct TransferHello {
    source_device_id: String,
}

impl Default for FlowEchoService {
    fn default() -> Self {
        Self::new(DEFAULT_BIND_ADDR, default_storage_dir())
    }
}

impl FlowEchoService {
    pub fn new(bind_addr: impl Into<String>, storage_dir: PathBuf) -> Self {
        let storage_dir = storage_dir;
        std::fs::create_dir_all(&storage_dir).expect("create storage dir");
        let paste_policy = default_paste_policy();
        let inbound_dir = storage_dir.join("inbound");
        std::fs::create_dir_all(&inbound_dir).expect("create inbound dir");
        Self {
            inner: Arc::new(FlowEchoServiceInner {
                bind_addr: bind_addr.into(),
                session_seq: AtomicU64::new(1),
                local_identity: RwLock::new(None),
                pairing: RwLock::new(PairingCoordinator::default()),
                paste_policy: RwLock::new(paste_policy),
                inbound_transfers: Mutex::new(
                    TransferCoordinator::new(inbound_dir).expect("inbound transfer coordinator"),
                ),
                outbound_transfers: Mutex::new(HashMap::new()),
                received_texts: RwLock::new(Vec::new()),
                received_files: RwLock::new(Vec::new()),
                listener: Mutex::new(None),
            }),
        }
    }

    pub fn shared() -> &'static FlowEchoService {
        static SERVICE: OnceLock<FlowEchoService> = OnceLock::new();
        SERVICE.get_or_init(FlowEchoService::default)
    }

    pub fn start_pairing(&self, req: StartPairingRequest) -> FlowResult<PairingChallenge> {
        self.set_local_identity(req.local_device_id.clone(), req.local_alias.clone())?;
        let endpoint = self.ensure_listener()?;
        let challenge = self
            .inner
            .pairing
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "pairing lock poisoned"))?
            .issue_challenge(
                PairChallengeRequest {
                    local_device_id: req.local_device_id,
                    local_alias: req.local_alias,
                    peer_ip: req.peer_ip.clone(),
                },
                now_ms(),
            )?;
        Ok(PairingChallenge {
            peer_ip: req.peer_ip,
            listen_port: port_from_endpoint(&endpoint)?,
            otp_code: challenge.otp_code,
            expires_at_ms: challenge.expires_at_ms,
            attempts_remaining: challenge.attempts_remaining,
        })
    }

    pub fn pair_device(&self, req: PairDeviceRequest) -> FlowResult<DeviceTrust> {
        if req.otp_code.trim().len() != 6 {
            return Err(FlowError::new(
                ErrorCode::PairOtpInvalid,
                "otp_code must be 6 digits",
            ));
        }
        self.set_local_identity(req.local_device_id.clone(), req.local_alias.clone())?;
        let local_endpoint = self.ensure_listener()?;
        let local_port = port_from_endpoint(&local_endpoint)?;
        let keypair = generate_handshake_keypair();

        let mut session = transport_adapter(TransportMode::Tcp).connect(&TransportEndpoint::new(
            format!("{}:{}", req.peer_ip, req.peer_port),
        ))?;
        session.send_frame(&TransportFrame::new(
            PAIR_AUTH_FRAME_TYPE,
            encode_payload(&PairingWireRequest {
                otp_code: req.otp_code,
                remote_device_id: req.local_device_id,
                remote_alias: req.local_alias,
                remote_public_key: hex_public_key(keypair.public_key),
                callback_port: local_port,
            })?,
        ))?;

        let response_frame = session.recv_frame()?;
        if response_frame.frame_type != PAIR_AUTH_RESPONSE_FRAME_TYPE {
            return Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "unexpected pairing response frame type",
            ));
        }
        let response: PairingWireResponse = decode_payload(&response_frame.payload)?;
        let session_key = derive_session_key(
            keypair.private_key,
            decode_public_key(&response.local_public_key)?,
            response.challenge_id.as_bytes(),
        )?;
        let trust = self
            .inner
            .pairing
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "pairing lock poisoned"))?
            .register_trusted_peer(
                req.peer_ip,
                req.peer_port,
                response.device_id,
                response.alias,
                session_key,
                response.session_key_id,
            );
        Ok(trust)
    }

    pub fn send_text(&self, req: SendTextRequest) -> FlowResult<TransferOutcome> {
        let local_identity = self.local_identity()?;
        let session_key = self.session_key_for_peer(&req.peer_ip, req.peer_port)?;
        let session_id = self.next_session_id("text");
        let resume_token = build_resume_token(&session_id, &session_id, &format!("{}:{}", req.peer_ip, req.peer_port));
        let hash = sha256_hex(req.text.as_bytes());
        let mut session = transport_adapter(TransportMode::Tcp).connect(&TransportEndpoint::new(
            format!("{}:{}", req.peer_ip, req.peer_port),
        ))?;
        self.send_transfer_hello(&mut session, &local_identity.device_id)?;
        send_encrypted_packet(
            &mut session,
            session_key,
            &TransferPacket::Text {
                payload_id: session_id.clone(),
                mime: "text/plain".to_string(),
                text: req.text.clone(),
                hash: hash.clone(),
            },
        )?;
        match recv_encrypted_packet(&mut session, session_key)? {
            TransferPacket::TextAck { payload_id, hash: ack_hash } => Ok(TransferOutcome {
                session_id: payload_id,
                resume_token,
                state: TransferState::Completed,
                bytes_transferred: req.text.len() as u64,
                total_bytes: req.text.len() as u64,
                missing_chunks: Vec::new(),
                message: if ack_hash == hash {
                    "text delivered".to_string()
                } else {
                    "text delivered with mismatched ack hash".to_string()
                },
            }),
            _ => Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "unexpected text transfer response",
            )),
        }
    }

    pub fn send_file(&self, req: SendFileRequest) -> FlowResult<TransferOutcome> {
        let session_id = self.next_session_id("file");
        let source_path = PathBuf::from(&req.file_path);
        let payload_id = file_payload_id(&source_path, &session_id);
        let resume_token = build_resume_token(
            &session_id,
            &payload_id,
            &format!("{}:{}", req.peer_ip, req.peer_port),
        );
        let plan = create_file_plan(&payload_id, &source_path, 256 * 1024)?;
        self.inner
            .outbound_transfers
            .lock()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "outbound transfer lock poisoned"))?
            .insert(
                resume_token.clone(),
                OutboundTransferRecord {
                    session_id: session_id.clone(),
                    source_path: source_path.clone(),
                    plan: plan.clone(),
                },
            );
        match self.send_file_inner(&session_id, &resume_token, &req.peer_ip, req.peer_port, &plan, &source_path) {
            Ok(outcome) => {
                if outcome.state == TransferState::Completed {
                    self.remove_outbound_transfer(&resume_token)?;
                }
                Ok(outcome)
            }
            Err(err) if matches!(err.code, ErrorCode::SessionClosed | ErrorCode::PeerUnreachable) => {
                Ok(TransferOutcome {
                    session_id,
                    resume_token,
                    state: TransferState::PendingResume,
                    bytes_transferred: 0,
                    total_bytes: plan.manifest.size,
                    missing_chunks: (0..plan.manifest.total_chunks).collect(),
                    message: "connection interrupted; use resume_transfer".to_string(),
                })
            }
            Err(err) => Err(err),
        }
    }

    pub fn resume_transfer(&self, req: ResumeTransferRequest) -> FlowResult<TransferOutcome> {
        let record = self
            .inner
            .outbound_transfers
            .lock()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "outbound transfer lock poisoned"))?
            .get(&req.resume_token)
            .cloned()
            .ok_or_else(|| FlowError::new(ErrorCode::TransferNotFound, "resume_token is unknown"))?;

        let local_identity = self.local_identity()?;
        let session_key = self.session_key_for_peer(&req.peer_ip, req.peer_port)?;
        let mut session = transport_adapter(TransportMode::Tcp).connect(&TransportEndpoint::new(
            format!("{}:{}", req.peer_ip, req.peer_port),
        ))?;
        self.send_transfer_hello(&mut session, &local_identity.device_id)?;
        send_encrypted_packet(
            &mut session,
            session_key,
            &TransferPacket::ResumeProbe {
                resume_token: req.resume_token.clone(),
            },
        )?;
        let resume_ack = expect_file_ack(recv_encrypted_packet(&mut session, session_key)?)?;
        let mut bytes_transferred = 0_u64;
        for index in &resume_ack.missing_chunks {
            let chunk = record
                .plan
                .chunks
                .get(*index as usize)
                .ok_or_else(|| FlowError::new(ErrorCode::InvalidChunk, "missing chunk plan"))?;
            let bytes = read_file_chunk(&record.source_path, chunk.offset, chunk.size)?;
            send_encrypted_packet(
                &mut session,
                session_key,
                &TransferPacket::FileChunk {
                    resume_token: req.resume_token.clone(),
                    index: *index,
                    bytes,
                },
            )?;
            let ack = expect_file_ack(recv_encrypted_packet(&mut session, session_key)?)?;
            if ack.last_error_code.is_some() {
                return Ok(TransferOutcome {
                    session_id: record.session_id,
                    resume_token: req.resume_token,
                    state: TransferState::PendingResume,
                    bytes_transferred,
                    total_bytes: record.plan.manifest.size,
                    missing_chunks: ack.missing_chunks,
                    message: "receiver rejected a resent chunk".to_string(),
                });
            }
            bytes_transferred += chunk.size as u64;
        }
        send_encrypted_packet(
            &mut session,
            session_key,
            &TransferPacket::FileFinish {
                resume_token: req.resume_token.clone(),
            },
        )?;
        match recv_encrypted_packet(&mut session, session_key)? {
            TransferPacket::FileComplete(completed) => {
                self.remove_outbound_transfer(&req.resume_token)?;
                Ok(TransferOutcome {
                    session_id: record.session_id,
                    resume_token: completed.resume_token,
                    state: TransferState::Completed,
                    bytes_transferred: record.plan.manifest.size,
                    total_bytes: record.plan.manifest.size,
                    missing_chunks: Vec::new(),
                    message: "file transfer resumed and completed".to_string(),
                })
            }
            TransferPacket::FileAck(ack) => Ok(TransferOutcome {
                session_id: record.session_id,
                resume_token: req.resume_token,
                state: TransferState::PendingResume,
                bytes_transferred,
                total_bytes: record.plan.manifest.size,
                missing_chunks: ack.missing_chunks,
                message: "file transfer still has missing chunks".to_string(),
            }),
            _ => Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "unexpected file resume response",
            )),
        }
    }

    pub fn latest_received_text(&self) -> Option<ReceivedTextPayload> {
        self.inner
            .received_texts
            .read()
            .ok()
            .and_then(|items| items.last().cloned())
    }

    pub fn latest_received_file(&self) -> Option<ReceivedFilePayload> {
        self.inner
            .received_files
            .read()
            .ok()
            .and_then(|items| items.last().cloned())
    }

    pub fn publish_clipboard(&self, req: PublishClipboardRequest) -> FlowResult<SyncAck> {
        if req.payload_manifest.payload_id.trim().is_empty() {
            return Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "payload_id is required",
            ));
        }
        Ok(SyncAck {
            ack_id: format!("ack-{}", req.payload_manifest.payload_id),
            accepted: true,
            reason: None,
        })
    }

    pub fn start_transfer(&self, req: StartTransferRequest) -> FlowResult<TransferSession> {
        if req.payload_id.trim().is_empty() || req.target_device.trim().is_empty() {
            return Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "payload_id and target_device are required",
            ));
        }
        let seq = self.inner.session_seq.fetch_add(1, Ordering::Relaxed);
        let session_id = format!("tx-{}-{}-{}", req.target_device, req.payload_id, seq);
        Ok(TransferSession {
            session_id: session_id.clone(),
            chunk_size: 256 * 1024,
            offset: 0,
            resume_token: build_resume_token(&session_id, &req.payload_id, &req.target_device),
            throughput_hint_kbps: 51200,
        })
    }

    pub fn apply_paste(&self, req: ApplyPasteRequest) -> FlowResult<PasteResult> {
        self.apply_paste_with_policy(req, self.current_paste_policy())
    }

    pub fn apply_paste_with_policy(
        &self,
        req: ApplyPasteRequest,
        policy: PastePolicy,
    ) -> FlowResult<PasteResult> {
        let decision = decide_route(req.mode, &policy, req.route_context.as_ref());
        Ok(PasteResult {
            applied: true,
            source: match decision.source {
                PasteSource::FlowEcho => PasteSource::FlowEcho,
                PasteSource::Native => PasteSource::Native,
            },
            restored_native_snapshot: decision.restored_native_snapshot,
            message: decision.message,
        })
    }

    pub fn set_paste_policy(&self, req: PastePolicy) -> FlowResult<SetPastePolicyResponse> {
        let mut guard = self
            .inner
            .paste_policy
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "policy lock poisoned"))?;
        *guard = req;
        Ok(SetPastePolicyResponse {
            saved: true,
            effective_at_ms: now_ms(),
        })
    }

    fn send_file_inner(
        &self,
        session_id: &str,
        resume_token: &str,
        peer_ip: &str,
        peer_port: u16,
        plan: &crate::transfer::TransferPlan,
        source_path: &PathBuf,
    ) -> FlowResult<TransferOutcome> {
        let local_identity = self.local_identity()?;
        let session_key = self.session_key_for_peer(peer_ip, peer_port)?;
        let mut session = transport_adapter(TransportMode::Tcp).connect(&TransportEndpoint::new(
            format!("{}:{}", peer_ip, peer_port),
        ))?;
        self.send_transfer_hello(&mut session, &local_identity.device_id)?;
        send_encrypted_packet(
            &mut session,
            session_key,
            &TransferPacket::FileOffer {
                plan: plan.clone(),
                resume_token: resume_token.to_string(),
            },
        )?;
        let _ = expect_file_ack(recv_encrypted_packet(&mut session, session_key)?)?;
        let mut bytes_transferred = 0_u64;
        for chunk in &plan.chunks {
            let bytes = read_file_chunk(source_path, chunk.offset, chunk.size)?;
            send_encrypted_packet(
                &mut session,
                session_key,
                &TransferPacket::FileChunk {
                    resume_token: resume_token.to_string(),
                    index: chunk.index,
                    bytes,
                },
            )?;
            let ack = expect_file_ack(recv_encrypted_packet(&mut session, session_key)?)?;
            if ack.last_error_code.is_some() {
                return Ok(TransferOutcome {
                    session_id: session_id.to_string(),
                    resume_token: resume_token.to_string(),
                    state: TransferState::PendingResume,
                    bytes_transferred,
                    total_bytes: plan.manifest.size,
                    missing_chunks: ack.missing_chunks,
                    message: "receiver rejected a chunk; use resume_transfer".to_string(),
                });
            }
            bytes_transferred += chunk.size as u64;
        }
        send_encrypted_packet(
            &mut session,
            session_key,
            &TransferPacket::FileFinish {
                resume_token: resume_token.to_string(),
            },
        )?;
        match recv_encrypted_packet(&mut session, session_key)? {
            TransferPacket::FileComplete(_) => Ok(TransferOutcome {
                session_id: session_id.to_string(),
                resume_token: resume_token.to_string(),
                state: TransferState::Completed,
                bytes_transferred: plan.manifest.size,
                total_bytes: plan.manifest.size,
                missing_chunks: Vec::new(),
                message: "file transfer completed".to_string(),
            }),
            TransferPacket::FileAck(ack) => Ok(TransferOutcome {
                session_id: session_id.to_string(),
                resume_token: resume_token.to_string(),
                state: TransferState::PendingResume,
                bytes_transferred,
                total_bytes: plan.manifest.size,
                missing_chunks: ack.missing_chunks,
                message: "file transfer incomplete; use resume_transfer".to_string(),
            }),
            _ => Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "unexpected file transfer response",
            )),
        }
    }

    fn ensure_listener(&self) -> FlowResult<TransportEndpoint> {
        let mut guard = self
            .inner
            .listener
            .lock()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "listener lock poisoned"))?;
        if let Some(runtime) = guard.as_ref() {
            return Ok(runtime.endpoint.clone());
        }

        let adapter = transport_adapter(TransportMode::Tcp);
        let listener = adapter.bind(&self.inner.bind_addr)?;
        let endpoint = listener.local_endpoint()?;
        let service = self.clone();
        thread::spawn(move || {
            loop {
                let session = match listener.accept() {
                    Ok(session) => session,
                    Err(_) => break,
                };
                let service = service.clone();
                thread::spawn(move || {
                    let _ = service.handle_session(session);
                });
            }
        });

        *guard = Some(ListenerRuntime {
            endpoint: endpoint.clone(),
        });
        Ok(endpoint)
    }

    fn handle_session(&self, mut session: TransportSession) -> FlowResult<()> {
        let first_frame = session.recv_frame()?;
        match first_frame.frame_type {
            PAIR_AUTH_FRAME_TYPE => {
                let peer_ip = peer_ip_from_endpoint(&session.peer_endpoint()?)?;
                let request: PairingWireRequest = decode_payload(&first_frame.payload)?;
                let response = self.accept_pair_request(&peer_ip, request)?;
                session.send_frame(&TransportFrame::new(
                    PAIR_AUTH_RESPONSE_FRAME_TYPE,
                    encode_payload(&response)?,
                ))
            }
            TRANSFER_HELLO_FRAME_TYPE => {
                let hello: TransferHello = decode_payload(&first_frame.payload)?;
                self.handle_transfer_stream(&mut session, hello)
            }
            _ => Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "unknown session bootstrap frame",
            )),
        }
    }

    fn handle_transfer_stream(
        &self,
        session: &mut TransportSession,
        hello: TransferHello,
    ) -> FlowResult<()> {
        let session_key = self
            .inner
            .pairing
            .read()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "pairing lock poisoned"))?
            .session_key_for_device(&hello.source_device_id)
            .ok_or_else(|| FlowError::new(ErrorCode::UntrustedDevice, "source device is not trusted"))?;
        let peer_ip = peer_ip_from_endpoint(&session.peer_endpoint()?)?;

        loop {
            match recv_encrypted_packet(session, session_key)? {
                TransferPacket::Text {
                    payload_id,
                    text,
                    hash,
                    ..
                } => {
                    if sha256_hex(text.as_bytes()) != hash {
                        return Err(FlowError::new(ErrorCode::HashMismatch, "text hash mismatch"));
                    }
                    self.inner
                        .received_texts
                        .write()
                        .map_err(|_| FlowError::new(ErrorCode::Internal, "text inbox lock poisoned"))?
                        .push(ReceivedTextPayload {
                            peer_ip,
                            payload_id: payload_id.clone(),
                            text,
                        });
                    send_encrypted_packet(
                        session,
                        session_key,
                        &TransferPacket::TextAck { payload_id, hash },
                    )?;
                    return Ok(());
                }
                TransferPacket::FileOffer { plan, resume_token } => {
                    let ack = self
                        .inner
                        .inbound_transfers
                        .lock()
                        .map_err(|_| FlowError::new(ErrorCode::Internal, "inbound transfer lock poisoned"))?
                        .accept_offer(plan, resume_token)?;
                    send_encrypted_packet(session, session_key, &TransferPacket::FileAck(ack))?;
                }
                TransferPacket::ResumeProbe { resume_token } => {
                    let ack = self
                        .inner
                        .inbound_transfers
                        .lock()
                        .map_err(|_| FlowError::new(ErrorCode::Internal, "inbound transfer lock poisoned"))?
                        .resume_state(&resume_token)?;
                    send_encrypted_packet(session, session_key, &TransferPacket::FileAck(ack))?;
                }
                TransferPacket::FileChunk {
                    resume_token,
                    index,
                    bytes,
                } => {
                    let response = match self
                        .inner
                        .inbound_transfers
                        .lock()
                        .map_err(|_| FlowError::new(ErrorCode::Internal, "inbound transfer lock poisoned"))?
                        .apply_chunk(&resume_token, index, &bytes)
                    {
                        Ok(ack) => TransferPacket::FileAck(ack),
                        Err(err) => TransferPacket::FileAck(
                            self.resume_state_for_inbound(&resume_token)?
                                .with_error(err.code),
                        ),
                    };
                    send_encrypted_packet(session, session_key, &response)?;
                }
                TransferPacket::FileFinish { resume_token } => {
                    let response = match self
                        .inner
                        .inbound_transfers
                        .lock()
                        .map_err(|_| FlowError::new(ErrorCode::Internal, "inbound transfer lock poisoned"))?
                        .finish(&resume_token)
                    {
                        Ok(completed) => {
                            self.record_received_file(&peer_ip, &completed)?;
                            TransferPacket::FileComplete(completed)
                        }
                        Err(err) => TransferPacket::FileAck(
                            self.resume_state_for_inbound(&resume_token)?
                                .with_error(err.code),
                        ),
                    };
                    send_encrypted_packet(session, session_key, &response)?;
                    return Ok(());
                }
                TransferPacket::TextAck { .. }
                | TransferPacket::FileAck(_)
                | TransferPacket::FileComplete(_) => {
                    return Err(FlowError::new(
                        ErrorCode::InvalidRequest,
                        "receiver cannot consume response packets",
                    ))
                }
            }
        }
    }

    fn accept_pair_request(
        &self,
        peer_ip: &str,
        request: PairingWireRequest,
    ) -> FlowResult<PairingWireResponse> {
        let acceptance: PairAcceptance = self
            .inner
            .pairing
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "pairing lock poisoned"))?
            .complete_network_pairing(
                peer_ip,
                request.callback_port,
                &request.remote_device_id,
                &request.remote_alias,
                &request.otp_code,
                &request.remote_public_key,
                now_ms(),
            )?;
        Ok(PairingWireResponse {
            challenge_id: acceptance.challenge_id,
            device_id: acceptance.local_device_id,
            alias: acceptance.local_alias,
            local_public_key: acceptance.local_public_key,
            session_key_id: acceptance.remote_trust.session_key_id,
        })
    }

    fn send_transfer_hello(&self, session: &mut TransportSession, device_id: &str) -> FlowResult<()> {
        session.send_frame(&TransportFrame::new(
            TRANSFER_HELLO_FRAME_TYPE,
            encode_payload(&TransferHello {
                source_device_id: device_id.to_string(),
            })?,
        ))
    }

    fn set_local_identity(&self, device_id: String, _alias: String) -> FlowResult<()> {
        let mut guard = self
            .inner
            .local_identity
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "identity lock poisoned"))?;
        *guard = Some(LocalIdentity { device_id });
        Ok(())
    }

    fn local_identity(&self) -> FlowResult<LocalIdentity> {
        self.inner
            .local_identity
            .read()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "identity lock poisoned"))?
            .clone()
            .ok_or_else(|| FlowError::new(ErrorCode::InvalidRequest, "local device identity is not set"))
    }

    fn session_key_for_peer(&self, peer_ip: &str, peer_port: u16) -> FlowResult<[u8; 32]> {
        self.inner
            .pairing
            .read()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "pairing lock poisoned"))?
            .session_key_for_endpoint(peer_ip, peer_port)
            .ok_or_else(|| FlowError::new(ErrorCode::UntrustedDevice, "peer is not trusted"))
    }

    fn record_received_file(&self, peer_ip: &str, completed: &TransferCompleted) -> FlowResult<()> {
        self.inner
            .received_files
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "file inbox lock poisoned"))?
            .push(ReceivedFilePayload {
                peer_ip: peer_ip.to_string(),
                payload_id: completed.payload_id.clone(),
                payload_hash: completed.payload_hash.clone(),
                file_path: completed.file_path.clone(),
            });
        Ok(())
    }

    fn resume_state_for_inbound(&self, resume_token: &str) -> FlowResult<TransferAck> {
        self.inner
            .inbound_transfers
            .lock()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "inbound transfer lock poisoned"))?
            .resume_state(resume_token)
    }

    fn remove_outbound_transfer(&self, resume_token: &str) -> FlowResult<()> {
        self.inner
            .outbound_transfers
            .lock()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "outbound transfer lock poisoned"))?
            .remove(resume_token);
        Ok(())
    }

    fn current_paste_policy(&self) -> PastePolicy {
        self.inner
            .paste_policy
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| default_paste_policy())
    }

    fn next_session_id(&self, prefix: &str) -> String {
        let seq = self.inner.session_seq.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}-{seq}")
    }
}

fn expect_file_ack(packet: TransferPacket) -> FlowResult<TransferAck> {
    match packet {
        TransferPacket::FileAck(ack) => Ok(ack),
        _ => Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "expected file ack packet",
        )),
    }
}

fn encode_payload<T: Serialize>(value: &T) -> FlowResult<Vec<u8>> {
    bincode::serialize(value)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "payload serialization failed"))
}

fn decode_payload<T: DeserializeOwned>(bytes: &[u8]) -> FlowResult<T> {
    bincode::deserialize(bytes)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "payload decode failed"))
}

fn port_from_endpoint(endpoint: &TransportEndpoint) -> FlowResult<u16> {
    endpoint
        .address
        .parse::<SocketAddr>()
        .map(|value| value.port())
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "invalid endpoint address"))
}

fn peer_ip_from_endpoint(endpoint: &TransportEndpoint) -> FlowResult<String> {
    endpoint
        .address
        .parse::<SocketAddr>()
        .map(|value| value.ip().to_string())
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "invalid peer endpoint address"))
}

fn read_file_chunk(path: &PathBuf, offset: u64, size: u32) -> FlowResult<Vec<u8>> {
    let mut file = File::open(path)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "source file is not readable"))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to seek source file"))?;
    let mut buffer = vec![0u8; size as usize];
    file.read_exact(&mut buffer)
        .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to read source file chunk"))?;
    Ok(buffer)
}

fn file_payload_id(path: &PathBuf, session_id: &str) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{session_id}-{name}"))
        .unwrap_or_else(|| format!("{session_id}-payload"))
}

fn hex_public_key(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_public_key(input: &str) -> FlowResult<[u8; 32]> {
    if input.len() != 64 {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "public key must be 64 hex chars",
        ));
    }
    let mut output = [0u8; 32];
    for (index, slot) in output.iter_mut().enumerate() {
        let start = index * 2;
        *slot = u8::from_str_radix(&input[start..start + 2], 16)
            .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "public key is not valid hex"))?;
    }
    Ok(output)
}

fn sha256_hex(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis() as u64
}

fn default_paste_policy() -> PastePolicy {
    PastePolicy {
        mode: PastePolicyMode::FlowEchoDefault,
        bypass_rules: vec![
            "password_field".to_string(),
            "rdp".to_string(),
            "terminal_high_risk".to_string(),
        ],
        app_scope: AppScope::AllApps,
    }
}

fn default_storage_dir() -> PathBuf {
    std::env::temp_dir().join("flowecho-runtime")
}
