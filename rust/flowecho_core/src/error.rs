use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use thiserror::Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u16)]
pub enum ErrorCode {
    InvalidRequest = 1001,
    InvalidVerifyCode = 1002,
    UntrustedDevice = 1003,
    UnsupportedTransport = 1004,
    PeerUnreachable = 1005,
    SessionClosed = 1006,
    PairOtpExpired = 1007,
    PairOtpInvalid = 1008,
    PairTimeout = 1009,
    HandshakeFailed = 1100,
    EncryptionFailed = 1101,
    DecryptionFailed = 1102,
    PasteBypassed = 1200,
    TransferNotFound = 1300,
    HashMismatch = 1301,
    InvalidChunk = 1302,
    TransferIncomplete = 1303,
    Internal = 9000,
}

#[derive(Debug, Error, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[error("{message}")]
pub struct FlowError {
    pub code: ErrorCode,
    pub message: String,
}

impl FlowError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub type FlowResult<T> = Result<T, FlowError>;
