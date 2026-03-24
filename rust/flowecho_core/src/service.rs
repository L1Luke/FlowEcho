use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{ErrorCode, FlowError, FlowResult};
use crate::paste_router::decide_route;
use crate::protocol::{
    ApplyPasteRequest, DeviceTrust, PairDeviceRequest, PastePolicy, PasteResult, PasteSource,
    PublishClipboardRequest, SetPastePolicyResponse, StartTransferRequest, SyncAck,
    TransferSession, TrustState,
};
use crate::transfer::build_resume_token;

static SESSION_SEQ: AtomicU64 = AtomicU64::new(1);
static PASTE_POLICY_STORE: OnceLock<RwLock<PastePolicy>> = OnceLock::new();

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
        let seq = SESSION_SEQ.fetch_add(1, Ordering::Relaxed);
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
        let store = self.policy_store();
        let mut guard = store
            .write()
            .map_err(|_| FlowError::new(ErrorCode::Internal, "policy lock poisoned"))?;
        *guard = req;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| FlowError::new(ErrorCode::Internal, "system clock failure"))?
            .as_millis() as u64;
        Ok(SetPastePolicyResponse {
            saved: true,
            effective_at_ms: now,
        })
    }

    fn default_paste_policy(&self) -> PastePolicy {
        PastePolicy {
            mode: crate::protocol::PastePolicyMode::FlowEchoDefault,
            bypass_rules: vec![
                "password_field".to_string(),
                "rdp".to_string(),
                "terminal_high_risk".to_string(),
            ],
            app_scope: crate::protocol::AppScope::AllApps,
        }
    }

    fn current_paste_policy(&self) -> PastePolicy {
        let store = self.policy_store();
        match store.read() {
            Ok(guard) => guard.clone(),
            Err(_) => self.default_paste_policy(),
        }
    }

    fn policy_store(&self) -> &'static RwLock<PastePolicy> {
        PASTE_POLICY_STORE.get_or_init(|| RwLock::new(self.default_paste_policy()))
    }
}
