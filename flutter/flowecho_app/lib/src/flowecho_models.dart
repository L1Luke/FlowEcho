enum TrustState { trusted, pending, revoked }

enum PayloadType { text, image, file }

enum ClipboardPriority { normal, high }

enum PasteMode { flowecho, nativeRestore }

enum PasteSource { flowecho, native }

enum PastePolicyMode { flowechoDefault, nativeDefault }

enum AppScope { allApps, allowList, denyList }

class PairDeviceRequest {
  PairDeviceRequest({
    required this.requestQr,
    required this.verifyCode,
  });

  final String requestQr;
  final String verifyCode;

  Map<String, Object?> toJson() => {
        "request_qr": requestQr,
        "verify_code": verifyCode,
      };
}

class DeviceTrust {
  DeviceTrust({
    required this.deviceId,
    required this.alias,
    required this.trustState,
    required this.sessionKeyId,
  });

  final String deviceId;
  final String alias;
  final TrustState trustState;
  final String sessionKeyId;

  factory DeviceTrust.fromJson(Map<String, Object?> json) {
    return DeviceTrust(
      deviceId: json["device_id"] as String,
      alias: json["alias"] as String,
      trustState: TrustState.values.firstWhere(
        (e) => e.name == json["trust_state"],
      ),
      sessionKeyId: json["session_key_id"] as String,
    );
  }
}

class PayloadManifest {
  PayloadManifest({
    required this.payloadId,
    required this.type,
    required this.mime,
    required this.size,
    required this.hash,
    required this.createdAt,
  });

  final String payloadId;
  final PayloadType type;
  final int size;
  final String mime;
  final String hash;
  final int createdAt;

  Map<String, Object?> toJson() => {
        "payload_id": payloadId,
        "type": type.name,
        "mime": mime,
        "size": size,
        "hash": hash,
        "created_at": createdAt,
      };
}

class PublishClipboardRequest {
  PublishClipboardRequest({
    required this.sourceDevice,
    required this.payloadManifest,
    required this.ttlMs,
    required this.priority,
  });

  final String sourceDevice;
  final PayloadManifest payloadManifest;
  final int ttlMs;
  final ClipboardPriority priority;

  Map<String, Object?> toJson() => {
        "source_device": sourceDevice,
        "payload_manifest": payloadManifest.toJson(),
        "ttl_ms": ttlMs,
        "priority": priority.name,
      };
}

class SyncAck {
  SyncAck({
    required this.ackId,
    required this.accepted,
    required this.reason,
  });

  final String ackId;
  final bool accepted;
  final String? reason;

  factory SyncAck.fromJson(Map<String, Object?> json) {
    return SyncAck(
      ackId: json["ack_id"] as String,
      accepted: json["accepted"] as bool,
      reason: json["reason"] as String?,
    );
  }
}

class StartTransferRequest {
  StartTransferRequest({
    required this.payloadId,
    required this.targetDevice,
  });

  final String payloadId;
  final String targetDevice;

  Map<String, Object?> toJson() => {
        "payload_id": payloadId,
        "target_device": targetDevice,
      };
}

class TransferSession {
  TransferSession({
    required this.sessionId,
    required this.chunkSize,
    required this.offset,
    required this.resumeToken,
    required this.throughputHintKbps,
  });

  final String sessionId;
  final int chunkSize;
  final int offset;
  final String resumeToken;
  final int throughputHintKbps;

  factory TransferSession.fromJson(Map<String, Object?> json) {
    return TransferSession(
      sessionId: json["session_id"] as String,
      chunkSize: json["chunk_size"] as int,
      offset: json["offset"] as int,
      resumeToken: json["resume_token"] as String,
      throughputHintKbps: json["throughput_hint_kbps"] as int,
    );
  }
}

class ApplyPasteRequest {
  ApplyPasteRequest({
    required this.mode,
    this.payloadId,
  });

  final PasteMode mode;
  final String? payloadId;

  Map<String, Object?> toJson() => {
        "mode": mode == PasteMode.nativeRestore ? "native_restore" : "flowecho",
        "payload_id": payloadId,
      };
}

class PasteResult {
  PasteResult({
    required this.applied,
    required this.source,
    required this.restoredNativeSnapshot,
    required this.message,
  });

  final bool applied;
  final PasteSource source;
  final bool restoredNativeSnapshot;
  final String message;

  factory PasteResult.fromJson(Map<String, Object?> json) {
    return PasteResult(
      applied: json["applied"] as bool,
      source: PasteSource.values.firstWhere((e) => e.name == json["source"]),
      restoredNativeSnapshot: json["restored_native_snapshot"] as bool,
      message: json["message"] as String,
    );
  }
}

class PastePolicy {
  PastePolicy({
    required this.mode,
    required this.bypassRules,
    required this.appScope,
  });

  final PastePolicyMode mode;
  final List<String> bypassRules;
  final AppScope appScope;

  Map<String, Object?> toJson() => {
        "mode": mode == PastePolicyMode.flowechoDefault
            ? "flowecho_default"
            : "native_default",
        "bypass_rules": bypassRules,
        "app_scope": switch (appScope) {
          AppScope.allApps => "all_apps",
          AppScope.allowList => "allow_list",
          AppScope.denyList => "deny_list",
        },
      };
}

class SetPastePolicyResult {
  SetPastePolicyResult({
    required this.saved,
    required this.effectiveAtMs,
  });

  final bool saved;
  final int effectiveAtMs;

  factory SetPastePolicyResult.fromJson(Map<String, Object?> json) {
    return SetPastePolicyResult(
      saved: json["saved"] as bool,
      effectiveAtMs: json["effective_at_ms"] as int,
    );
  }
}

class FlowEchoRpcError implements Exception {
  FlowEchoRpcError({
    required this.code,
    required this.message,
  });

  final int code;
  final String message;

  @override
  String toString() => "FlowEchoRpcError(code: $code, message: $message)";
}
