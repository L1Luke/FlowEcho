use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PairDeviceRequest {
    pub request_qr: String,
    pub verify_code: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TrustState {
    #[serde(rename = "trusted")]
    Trusted,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "revoked")]
    Revoked,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeviceTrust {
    pub device_id: String,
    pub alias: String,
    pub trust_state: TrustState,
    pub session_key_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PayloadType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PayloadManifest {
    pub payload_id: String,
    #[serde(rename = "type")]
    pub payload_type: PayloadType,
    pub mime: String,
    pub size: u64,
    pub hash: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClipboardPriority {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublishClipboardRequest {
    pub source_device: String,
    pub payload_manifest: PayloadManifest,
    pub ttl_ms: u64,
    pub priority: ClipboardPriority,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncAck {
    pub ack_id: String,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct StartTransferRequest {
    pub payload_id: String,
    pub target_device: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferSession {
    pub session_id: String,
    pub chunk_size: u32,
    pub offset: u64,
    pub resume_token: String,
    pub throughput_hint_kbps: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PasteMode {
    #[serde(rename = "flowecho")]
    FlowEcho,
    #[serde(rename = "native_restore")]
    NativeRestore,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplyPasteRequest {
    pub mode: PasteMode,
    pub payload_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PasteSource {
    #[serde(rename = "flowecho")]
    FlowEcho,
    #[serde(rename = "native")]
    Native,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PasteResult {
    pub applied: bool,
    pub source: PasteSource,
    pub restored_native_snapshot: bool,
    pub message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PastePolicyMode {
    #[serde(rename = "flowecho_default")]
    FlowEchoDefault,
    #[serde(rename = "native_default")]
    NativeDefault,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppScope {
    #[serde(rename = "all_apps")]
    AllApps,
    #[serde(rename = "allow_list")]
    AllowList,
    #[serde(rename = "deny_list")]
    DenyList,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PastePolicy {
    pub mode: PastePolicyMode,
    pub bypass_rules: Vec<String>,
    pub app_scope: AppScope,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetPastePolicyResponse {
    pub saved: bool,
    pub effective_at_ms: u64,
}
