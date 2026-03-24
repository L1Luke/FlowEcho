use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{ErrorCode, FlowError, FlowResult};

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

pub fn create_plan(payload_id: &str, bytes: &[u8], chunk_size: u32) -> FlowResult<TransferPlan> {
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

pub fn build_resume_token(session_id: &str, payload_id: &str, target_device: &str) -> String {
    let material = format!("{session_id}:{payload_id}:{target_device}");
    format!("resume-{}", sha256_hex(material.as_bytes()))
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

fn sha256_hex(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
