use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ErrorCode, FlowError, FlowResult};
use crate::protocol::{
    ApplyPasteRequest, DeviceTrust, PairDeviceRequest, PastePolicy, PasteResult, PasteSource,
    PublishClipboardRequest, SetPastePolicyResponse, StartTransferRequest, SyncAck,
    TransferSession, TrustState,
};

pub struct FlowEchoService;

impl Default for FlowEchoService {
    fn default() -> Self {
        Self
    }
}

impl FlowEchoService {
    pub fn pair_device(&self, req: PairDeviceRequest) -> FlowResult<DeviceTrust> {
        if req.verify_code.trim().len() < 4 {
            return Err(FlowError::new(
                ErrorCode::InvalidVerifyCode,
                "verify_code is too short",
            ));
        }
        Ok(DeviceTrust {
            device_id: format!("dev-{}", req.request_qr),
            alias: "Paired Device".to_string(),
            trust_state: TrustState::Trusted,
            session_key_id: "session-bootstrap".to_string(),
        })
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
        Ok(TransferSession {
            session_id: format!("tx-{}-{}", req.target_device, req.payload_id),
            chunk_size: 256 * 1024,
            offset: 0,
            resume_token: format!("resume-{}", req.payload_id),
            throughput_hint_kbps: 51200,
        })
    }

    pub fn apply_paste(&self, req: ApplyPasteRequest) -> FlowResult<PasteResult> {
        match req.mode {
            crate::protocol::PasteMode::FlowEcho => Ok(PasteResult {
                applied: true,
                source: PasteSource::FlowEcho,
                restored_native_snapshot: false,
                message: "FlowEcho payload applied".to_string(),
            }),
            crate::protocol::PasteMode::NativeRestore => Ok(PasteResult {
                applied: true,
                source: PasteSource::Native,
                restored_native_snapshot: true,
                message: "Native clipboard restored".to_string(),
            }),
        }
    }

    pub fn set_paste_policy(&self, _req: PastePolicy) -> FlowResult<SetPastePolicyResponse> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "system clock failure"))?
            .as_millis() as u64;
        Ok(SetPastePolicyResponse {
            saved: true,
            effective_at_ms: now,
        })
    }
}
