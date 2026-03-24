use flowecho_core::protocol::{
    AppScope, ApplyPasteRequest, PasteMode, PastePolicy, PastePolicyMode, PasteRouteContext,
    PasteSource, Platform, PressedHotkey,
};
use flowecho_core::service::FlowEchoService;

fn default_policy() -> PastePolicy {
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

#[test]
fn desktop_default_paste_routes_to_flowecho() {
    let service = FlowEchoService::default();
    let result = service
        .apply_paste_with_policy(
            ApplyPasteRequest {
                mode: PasteMode::FlowEcho,
                payload_id: Some("payload-1".to_string()),
                route_context: Some(PasteRouteContext {
                    platform: Platform::MacOs,
                    hotkey: PressedHotkey::DefaultPaste,
                    is_password_field: false,
                    is_remote_session: false,
                    is_terminal_session: false,
                    is_in_app_entry: true,
                    app_in_scope: true,
                }),
            },
            default_policy(),
        )
        .expect("apply");
    assert_eq!(result.source, PasteSource::FlowEcho);
}

#[test]
fn shift_hotkey_forces_native_restore() {
    let service = FlowEchoService::default();
    let result = service
        .apply_paste_with_policy(
            ApplyPasteRequest {
                mode: PasteMode::FlowEcho,
                payload_id: Some("payload-1".to_string()),
                route_context: Some(PasteRouteContext {
                    platform: Platform::Windows,
                    hotkey: PressedHotkey::NativeFallbackPaste,
                    is_password_field: false,
                    is_remote_session: false,
                    is_terminal_session: false,
                    is_in_app_entry: true,
                    app_in_scope: true,
                }),
            },
            default_policy(),
        )
        .expect("apply");
    assert_eq!(result.source, PasteSource::Native);
    assert!(result.restored_native_snapshot);
}

#[test]
fn sensitive_context_bypasses_to_native() {
    let service = FlowEchoService::default();
    let result = service
        .apply_paste_with_policy(
            ApplyPasteRequest {
                mode: PasteMode::FlowEcho,
                payload_id: Some("payload-1".to_string()),
                route_context: Some(PasteRouteContext {
                    platform: Platform::Windows,
                    hotkey: PressedHotkey::DefaultPaste,
                    is_password_field: true,
                    is_remote_session: false,
                    is_terminal_session: false,
                    is_in_app_entry: true,
                    app_in_scope: true,
                }),
            },
            default_policy(),
        )
        .expect("apply");
    assert_eq!(result.source, PasteSource::Native);
}

#[test]
fn ios_outside_app_entry_must_use_native() {
    let service = FlowEchoService::default();
    let result = service
        .apply_paste_with_policy(
            ApplyPasteRequest {
                mode: PasteMode::FlowEcho,
                payload_id: Some("payload-1".to_string()),
                route_context: Some(PasteRouteContext {
                    platform: Platform::Ios,
                    hotkey: PressedHotkey::DefaultPaste,
                    is_password_field: false,
                    is_remote_session: false,
                    is_terminal_session: false,
                    is_in_app_entry: false,
                    app_in_scope: true,
                }),
            },
            default_policy(),
        )
        .expect("apply");
    assert_eq!(result.source, PasteSource::Native);
}

#[test]
fn ios_in_app_entry_can_use_flowecho() {
    let service = FlowEchoService::default();
    let result = service
        .apply_paste_with_policy(
            ApplyPasteRequest {
                mode: PasteMode::FlowEcho,
                payload_id: Some("payload-1".to_string()),
                route_context: Some(PasteRouteContext {
                    platform: Platform::Ios,
                    hotkey: PressedHotkey::DefaultPaste,
                    is_password_field: false,
                    is_remote_session: false,
                    is_terminal_session: false,
                    is_in_app_entry: true,
                    app_in_scope: true,
                }),
            },
            default_policy(),
        )
        .expect("apply");
    assert_eq!(result.source, PasteSource::FlowEcho);
}
