use crate::protocol::{
    PasteMode, PastePolicy, PastePolicyMode, PasteRouteContext, PasteSource, Platform,
    PressedHotkey,
};

pub struct RouteDecision {
    pub source: PasteSource,
    pub restored_native_snapshot: bool,
    pub message: String,
}

pub fn decide_route(
    mode: PasteMode,
    policy: &PastePolicy,
    context: Option<&PasteRouteContext>,
) -> RouteDecision {
    if mode == PasteMode::NativeRestore {
        return native("Native fallback hotkey", true);
    }

    let Some(ctx) = context else {
        return match policy.mode {
            PastePolicyMode::FlowEchoDefault => flowecho("FlowEcho default route"),
            PastePolicyMode::NativeDefault => native("Native default route", false),
        };
    };

    if ctx.hotkey == PressedHotkey::NativeFallbackPaste {
        return native("Native fallback hotkey", true);
    }

    if ctx.platform == Platform::Ios && !ctx.is_in_app_entry {
        // iOS does not support cross-app global paste takeover.
        return native("iOS only supports in-app equivalent paste entry", false);
    }

    if !ctx.app_in_scope {
        return native("App scope bypass", false);
    }

    if has_rule(policy, "password_field") && ctx.is_password_field {
        return native("Sensitive password field bypass", false);
    }
    if has_rule(policy, "rdp") && ctx.is_remote_session {
        return native("Remote desktop bypass", false);
    }
    if has_rule(policy, "terminal_high_risk") && ctx.is_terminal_session {
        return native("High risk terminal bypass", false);
    }

    match policy.mode {
        PastePolicyMode::FlowEchoDefault => flowecho("FlowEcho default route"),
        PastePolicyMode::NativeDefault => native("Native default route", false),
    }
}

fn has_rule(policy: &PastePolicy, key: &str) -> bool {
    policy.bypass_rules.iter().any(|rule| rule == key)
}

fn flowecho(message: &str) -> RouteDecision {
    RouteDecision {
        source: PasteSource::FlowEcho,
        restored_native_snapshot: false,
        message: message.to_string(),
    }
}

fn native(message: &str, restored_native_snapshot: bool) -> RouteDecision {
    RouteDecision {
        source: PasteSource::Native,
        restored_native_snapshot,
        message: message.to_string(),
    }
}
