use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct StartPairingRequest {
    pub local_device_id: String,
    pub local_alias: String,
    pub peer_ip: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PairingChallenge {
    pub peer_ip: String,
    pub listen_port: u16,
    pub otp_code: String,
    pub expires_at_ms: u64,
    pub attempts_remaining: u8,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PairDeviceRequest {
    pub peer_ip: String,
    pub peer_port: u16,
    pub otp_code: String,
    pub local_device_id: String,
    pub local_alias: String,
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
pub struct SendTextRequest {
    pub peer_ip: String,
    pub peer_port: u16,
    pub text: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SendFileRequest {
    pub peer_ip: String,
    pub peer_port: u16,
    pub file_path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResumeTransferRequest {
    pub peer_ip: String,
    pub peer_port: u16,
    pub resume_token: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransferState {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "pending_resume")]
    PendingResume,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferOutcome {
    pub session_id: String,
    pub resume_token: String,
    pub state: TransferState,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub missing_chunks: Vec<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PasteMode {
    #[serde(rename = "flowecho")]
    FlowEcho,
    #[serde(rename = "native_restore")]
    NativeRestore,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Platform {
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "macos")]
    MacOs,
    #[serde(rename = "ios")]
    Ios,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PressedHotkey {
    #[serde(rename = "default_paste")]
    DefaultPaste,
    #[serde(rename = "native_fallback_paste")]
    NativeFallbackPaste,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PasteRouteContext {
    pub platform: Platform,
    pub hotkey: PressedHotkey,
    pub is_password_field: bool,
    pub is_remote_session: bool,
    pub is_terminal_session: bool,
    pub is_in_app_entry: bool,
    pub app_in_scope: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplyPasteRequest {
    pub mode: PasteMode,
    pub payload_id: Option<String>,
    #[serde(default)]
    pub route_context: Option<PasteRouteContext>,
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
