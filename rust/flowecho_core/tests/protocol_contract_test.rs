use flowecho_core::protocol::{
    AppScope, ClipboardPriority, PairDeviceRequest, PastePolicy, PastePolicyMode, PayloadManifest,
    PayloadType, PublishClipboardRequest,
};
use serde_json::{json, Value};

#[test]
fn payload_manifest_fields_are_stable() {
    let manifest = PayloadManifest {
        payload_id: "p-001".to_string(),
        payload_type: PayloadType::Text,
        mime: "text/plain".to_string(),
        size: 12,
        hash: "abc123".to_string(),
        created_at: 1_700_000_000_000,
    };
    let value = serde_json::to_value(manifest).expect("serialize payload manifest");
    assert_eq!(
        value,
        json!({
            "payload_id": "p-001",
            "type": "text",
            "mime": "text/plain",
            "size": 12,
            "hash": "abc123",
            "created_at": 1700000000000_u64,
        })
    );
}

#[test]
fn pair_device_request_fields_are_stable() {
    let req = PairDeviceRequest {
        request_qr: "qr://flowecho".to_string(),
        verify_code: "8848".to_string(),
    };
    let value = serde_json::to_value(req).expect("serialize pair request");
    assert_eq!(
        value,
        json!({
            "request_qr": "qr://flowecho",
            "verify_code": "8848",
        })
    );
}

#[test]
fn publish_clipboard_contract_roundtrip() {
    let req_json = json!({
        "source_device": "dev-a",
        "payload_manifest": {
            "payload_id": "pid-1",
            "type": "file",
            "mime": "application/octet-stream",
            "size": 1024,
            "hash": "deadbeef",
            "created_at": 1700000000001_u64
        },
        "ttl_ms": 30_000,
        "priority": "high"
    });
    let req: PublishClipboardRequest =
        serde_json::from_value(req_json.clone()).expect("deserialize publish request");
    assert_eq!(req.source_device, "dev-a");
    assert_eq!(req.priority, ClipboardPriority::High);
    let encoded = serde_json::to_value(req).expect("serialize publish request");
    assert_eq!(encoded, req_json);
}

#[test]
fn set_paste_policy_contract_roundtrip() {
    let policy = PastePolicy {
        mode: PastePolicyMode::FlowEchoDefault,
        bypass_rules: vec![
            "password_field".to_string(),
            "rdp".to_string(),
            "terminal_high_risk".to_string(),
        ],
        app_scope: AppScope::AllApps,
    };
    let value = serde_json::to_value(policy).expect("serialize policy");
    assert_eq!(value["mode"], Value::String("flowecho_default".to_string()));
    assert_eq!(value["app_scope"], Value::String("all_apps".to_string()));
    assert!(value["bypass_rules"].is_array());
}
