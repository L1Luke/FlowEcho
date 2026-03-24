use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::crypto::{open, seal};
use crate::error::{ErrorCode, FlowError, FlowResult};
use crate::transport::{TransportFrame, TransportSession};

const ENCRYPTED_TRANSFER_FRAME_TYPE: u8 = 0x21;
const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferManifest {
    pub payload_id: String,
    pub size: u64,
    pub chunk_size: u32,
    pub total_chunks: u32,
    pub hash: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferChunkPlan {
    pub index: u32,
    pub offset: u64,
    pub size: u32,
    pub hash: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferPlan {
    pub manifest: TransferManifest,
    pub chunks: Vec<TransferChunkPlan>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferAck {
    pub resume_token: String,
    pub received_bitmap: Vec<u8>,
    pub missing_chunks: Vec<u32>,
    pub complete: bool,
    pub last_error_code: Option<ErrorCode>,
}

impl TransferAck {
    pub fn with_error(mut self, code: ErrorCode) -> Self {
        self.last_error_code = Some(code);
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferCompleted {
    pub resume_token: String,
    pub payload_id: String,
    pub file_path: PathBuf,
    pub payload_hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransferPacket {
    Text {
        payload_id: String,
        mime: String,
        text: String,
        hash: String,
    },
    TextAck {
        payload_id: String,
        hash: String,
    },
    FileOffer {
        plan: TransferPlan,
        resume_token: String,
    },
    ResumeProbe {
        resume_token: String,
    },
    FileChunk {
        resume_token: String,
        index: u32,
        bytes: Vec<u8>,
    },
    FileAck(TransferAck),
    FileFinish {
        resume_token: String,
    },
    FileComplete(TransferCompleted),
}

pub fn create_plan(payload_id: &str, bytes: &[u8], chunk_size: u32) -> FlowResult<TransferPlan> {
    validate_transfer_inputs(payload_id, chunk_size)?;

    let mut chunks = Vec::new();
    let mut offset = 0_u64;
    for (index, chunk) in bytes.chunks(chunk_size as usize).enumerate() {
        chunks.push(TransferChunkPlan {
            index: index as u32,
            offset,
            size: chunk.len() as u32,
            hash: sha256_hex(chunk),
        });
        offset += chunk.len() as u64;
    }

    let manifest = TransferManifest {
        payload_id: payload_id.to_string(),
        size: bytes.len() as u64,
        chunk_size,
        total_chunks: chunks.len() as u32,
        hash: sha256_hex(bytes),
    };
    Ok(TransferPlan { manifest, chunks })
}

pub fn create_file_plan(payload_id: &str, path: &Path, chunk_size: u32) -> FlowResult<TransferPlan> {
    validate_transfer_inputs(payload_id, chunk_size)?;

    let mut file = File::open(path)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "source file is not readable"))?;
    let mut overall = Sha256::new();
    let mut chunks = Vec::new();
    let mut offset = 0_u64;
    let mut index = 0_u32;
    let mut buffer = vec![0u8; chunk_size as usize];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to read source file"))?;
        if read == 0 {
            break;
        }

        let chunk = &buffer[..read];
        overall.update(chunk);
        chunks.push(TransferChunkPlan {
            index,
            offset,
            size: read as u32,
            hash: sha256_hex(chunk),
        });
        offset += read as u64;
        index += 1;
    }

    let manifest = TransferManifest {
        payload_id: payload_id.to_string(),
        size: offset,
        chunk_size,
        total_chunks: chunks.len() as u32,
        hash: sha256_digest_hex(overall.finalize()),
    };
    Ok(TransferPlan { manifest, chunks })
}

pub fn build_resume_token(session_id: &str, payload_id: &str, target_device: &str) -> String {
    let material = format!("{session_id}:{payload_id}:{target_device}");
    format!("resume-{}", sha256_hex(material.as_bytes()))
}

pub fn send_encrypted_packet(
    session: &mut TransportSession,
    session_key: [u8; 32],
    packet: &TransferPacket,
) -> FlowResult<()> {
    let plaintext = bincode::serialize(packet)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "transfer packet serialization failed"))?;
    let (ciphertext, nonce) = seal(session_key, &plaintext)?;
    let mut payload = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);
    session.send_frame(&TransportFrame::new(ENCRYPTED_TRANSFER_FRAME_TYPE, payload))
}

pub fn recv_encrypted_packet(
    session: &mut TransportSession,
    session_key: [u8; 32],
) -> FlowResult<TransferPacket> {
    let frame = session.recv_frame()?;
    if frame.frame_type != ENCRYPTED_TRANSFER_FRAME_TYPE {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "unexpected transfer frame type",
        ));
    }
    if frame.payload.len() < NONCE_SIZE {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "encrypted transfer frame is missing nonce",
        ));
    }

    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&frame.payload[..NONCE_SIZE]);
    let plaintext = open(session_key, &frame.payload[NONCE_SIZE..], nonce)?;
    bincode::deserialize(&plaintext)
        .map_err(|_| FlowError::new(ErrorCode::InvalidRequest, "transfer packet decode failed"))
}

pub struct TransferCoordinator {
    storage_dir: PathBuf,
    inbound: HashMap<String, FileTransferSink>,
}

impl TransferCoordinator {
    pub fn new(storage_dir: PathBuf) -> FlowResult<Self> {
        fs::create_dir_all(&storage_dir)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to prepare transfer storage"))?;
        Ok(Self {
            storage_dir,
            inbound: HashMap::new(),
        })
    }

    pub fn accept_offer(&mut self, plan: TransferPlan, resume_token: String) -> FlowResult<TransferAck> {
        if resume_token.trim().is_empty() {
            return Err(FlowError::new(
                ErrorCode::InvalidRequest,
                "resume_token is required",
            ));
        }
        if !self.inbound.contains_key(&resume_token) {
            let file_path = self.storage_dir.join(format!("{}.part", sha256_hex(resume_token.as_bytes())));
            let sink = FileTransferSink::new(plan, file_path, resume_token.clone())?;
            self.inbound.insert(resume_token.clone(), sink);
        }
        self.resume_state(&resume_token)
    }

    pub fn resume_state(&self, resume_token: &str) -> FlowResult<TransferAck> {
        let sink = self
            .inbound
            .get(resume_token)
            .ok_or_else(|| FlowError::new(ErrorCode::TransferNotFound, "transfer session not found"))?;
        Ok(sink.ack(None))
    }

    pub fn apply_chunk(
        &mut self,
        resume_token: &str,
        index: u32,
        bytes: &[u8],
    ) -> FlowResult<TransferAck> {
        let sink = self
            .inbound
            .get_mut(resume_token)
            .ok_or_else(|| FlowError::new(ErrorCode::TransferNotFound, "transfer session not found"))?;
        sink.apply_chunk(index, bytes)
    }

    pub fn finish(&mut self, resume_token: &str) -> FlowResult<TransferCompleted> {
        let sink = self
            .inbound
            .get_mut(resume_token)
            .ok_or_else(|| FlowError::new(ErrorCode::TransferNotFound, "transfer session not found"))?;
        sink.finish(resume_token)
    }
}

pub struct TransferAssembler {
    manifest: TransferManifest,
    chunk_plan_by_index: HashMap<u32, TransferChunkPlan>,
    received: BTreeMap<u32, Vec<u8>>,
    acknowledged: BTreeSet<u32>,
}

impl TransferAssembler {
    pub fn new(plan: TransferPlan) -> Self {
        let chunk_plan_by_index = plan
            .chunks
            .iter()
            .map(|chunk| (chunk.index, chunk.clone()))
            .collect();
        Self {
            manifest: plan.manifest,
            chunk_plan_by_index,
            received: BTreeMap::new(),
            acknowledged: BTreeSet::new(),
        }
    }

    pub fn apply_chunk(&mut self, index: u32, data: &[u8]) -> FlowResult<()> {
        let expected = self.chunk_plan_by_index.get(&index).ok_or_else(|| {
            FlowError::new(ErrorCode::InvalidChunk, "chunk index is out of range")
        })?;

        if expected.size != data.len() as u32 {
            return Err(FlowError::new(
                ErrorCode::InvalidChunk,
                "chunk size mismatch",
            ));
        }
        if expected.hash != sha256_hex(data) {
            return Err(FlowError::new(
                ErrorCode::HashMismatch,
                "chunk hash mismatch",
            ));
        }

        self.received.insert(index, data.to_vec());
        self.acknowledged.insert(index);
        Ok(())
    }

    pub fn next_missing_chunk(&self) -> Option<u32> {
        (0..self.manifest.total_chunks).find(|index| !self.acknowledged.contains(index))
    }

    pub fn missing_chunks(&self) -> Vec<u32> {
        (0..self.manifest.total_chunks)
            .filter(|index| !self.acknowledged.contains(index))
            .collect()
    }

    pub fn resume_offset(&self) -> u64 {
        let mut offset = 0_u64;
        for index in 0..self.manifest.total_chunks {
            if let Some(chunk) = self.chunk_plan_by_index.get(&index) {
                if self.acknowledged.contains(&index) {
                    offset += chunk.size as u64;
                    continue;
                }
            }
            break;
        }
        offset
    }

    pub fn finish(self) -> FlowResult<Vec<u8>> {
        if !self.missing_chunks().is_empty() {
            return Err(FlowError::new(
                ErrorCode::TransferIncomplete,
                "transfer is incomplete",
            ));
        }

        let mut merged = Vec::with_capacity(self.manifest.size as usize);
        for index in 0..self.manifest.total_chunks {
            let chunk = self.received.get(&index).ok_or_else(|| {
                FlowError::new(ErrorCode::TransferIncomplete, "missing chunk during merge")
            })?;
            merged.extend_from_slice(chunk);
        }

        if sha256_hex(&merged) != self.manifest.hash {
            return Err(FlowError::new(
                ErrorCode::HashMismatch,
                "payload hash mismatch",
            ));
        }
        Ok(merged)
    }

    pub fn manifest(&self) -> &TransferManifest {
        &self.manifest
    }
}

struct FileTransferSink {
    manifest: TransferManifest,
    chunk_plan_by_index: HashMap<u32, TransferChunkPlan>,
    acknowledged: BTreeSet<u32>,
    file_path: PathBuf,
    resume_token: String,
}

impl FileTransferSink {
    fn new(plan: TransferPlan, file_path: PathBuf, resume_token: String) -> FlowResult<Self> {
        let chunk_plan_by_index = plan
            .chunks
            .iter()
            .map(|chunk| (chunk.index, chunk.clone()))
            .collect();
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(&file_path)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to create transfer sink"))?;
        file.set_len(plan.manifest.size)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to size transfer sink"))?;
        Ok(Self {
            manifest: plan.manifest,
            chunk_plan_by_index,
            acknowledged: BTreeSet::new(),
            file_path,
            resume_token,
        })
    }

    fn apply_chunk(&mut self, index: u32, bytes: &[u8]) -> FlowResult<TransferAck> {
        let expected = self.chunk_plan_by_index.get(&index).ok_or_else(|| {
            FlowError::new(ErrorCode::InvalidChunk, "chunk index is out of range")
        })?;
        if expected.size != bytes.len() as u32 {
            return Err(FlowError::new(
                ErrorCode::InvalidChunk,
                "chunk size mismatch",
            ));
        }
        if expected.hash != sha256_hex(bytes) {
            return Err(FlowError::new(
                ErrorCode::HashMismatch,
                "chunk hash mismatch",
            ));
        }

        let mut file = OpenOptions::new()
            .write(true)
            .open(&self.file_path)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to open transfer sink"))?;
        file.seek(SeekFrom::Start(expected.offset))
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to seek transfer sink"))?;
        file.write_all(bytes)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to write transfer chunk"))?;
        self.acknowledged.insert(index);
        Ok(self.ack(None))
    }

    fn finish(&mut self, resume_token: &str) -> FlowResult<TransferCompleted> {
        let ack = self.ack(None);
        if !ack.missing_chunks.is_empty() {
            return Err(FlowError::new(
                ErrorCode::TransferIncomplete,
                "transfer is incomplete",
            ));
        }

        let mut file = File::open(&self.file_path)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to open completed transfer"))?;
        let actual_hash = sha256_reader_hex(&mut file)?;
        if actual_hash != self.manifest.hash {
            return Err(FlowError::new(
                ErrorCode::HashMismatch,
                "payload hash mismatch",
            ));
        }

        Ok(TransferCompleted {
            resume_token: resume_token.to_string(),
            payload_id: self.manifest.payload_id.clone(),
            file_path: self.file_path.clone(),
            payload_hash: actual_hash,
            size: self.manifest.size,
        })
    }

    fn ack(&self, last_error_code: Option<ErrorCode>) -> TransferAck {
        let missing_chunks = self.missing_chunks();
        TransferAck {
            resume_token: self.resume_token.clone(),
            received_bitmap: self.received_bitmap(),
            complete: missing_chunks.is_empty(),
            missing_chunks,
            last_error_code,
        }
    }

    fn missing_chunks(&self) -> Vec<u32> {
        (0..self.manifest.total_chunks)
            .filter(|index| !self.acknowledged.contains(index))
            .collect()
    }

    fn received_bitmap(&self) -> Vec<u8> {
        let mut bitmap = vec![0u8; self.manifest.total_chunks.div_ceil(8) as usize];
        for index in &self.acknowledged {
            let byte_index = (*index / 8) as usize;
            let bit_index = (*index % 8) as u8;
            bitmap[byte_index] |= 1 << bit_index;
        }
        bitmap
    }
}

fn validate_transfer_inputs(payload_id: &str, chunk_size: u32) -> FlowResult<()> {
    if payload_id.trim().is_empty() {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "payload_id is required",
        ));
    }
    if chunk_size == 0 {
        return Err(FlowError::new(
            ErrorCode::InvalidRequest,
            "chunk_size must be > 0",
        ));
    }
    Ok(())
}

fn sha256_reader_hex(reader: &mut impl Read) -> FlowResult<String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "failed to hash transfer payload"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(sha256_digest_hex(hasher.finalize()))
}

fn sha256_hex(input: &[u8]) -> String {
    sha256_digest_hex(Sha256::digest(input))
}

fn sha256_digest_hex(digest: impl AsRef<[u8]>) -> String {
    let digest = digest.as_ref();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
