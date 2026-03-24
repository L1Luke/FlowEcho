use std::collections::HashSet;

use flowecho_core::error::{ErrorCode, FlowError};

#[test]
fn error_codes_are_unique_and_stable() {
    let codes = [
        ErrorCode::InvalidRequest as u16,
        ErrorCode::InvalidVerifyCode as u16,
        ErrorCode::UntrustedDevice as u16,
        ErrorCode::HandshakeFailed as u16,
        ErrorCode::EncryptionFailed as u16,
        ErrorCode::DecryptionFailed as u16,
        ErrorCode::PasteBypassed as u16,
        ErrorCode::TransferNotFound as u16,
        ErrorCode::Internal as u16,
    ];
    let uniq: HashSet<u16> = codes.into_iter().collect();
    assert_eq!(uniq.len(), codes.len(), "duplicate error codes found");
    assert!(uniq.contains(&1001));
    assert!(uniq.contains(&1102));
    assert!(uniq.contains(&9000));
}

#[test]
fn flow_error_serializes_code_and_message() {
    let err = FlowError::new(ErrorCode::InvalidRequest, "invalid JSON request");
    let value = serde_json::to_value(err).expect("serialize flow error");
    assert_eq!(value["code"], 1001);
    assert_eq!(value["message"], "invalid JSON request");
}
