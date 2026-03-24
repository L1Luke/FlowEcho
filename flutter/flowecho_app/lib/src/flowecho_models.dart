enum TrustState { trusted, pending, revoked }

enum PayloadType { text, image, file }

enum ClipboardPriority { normal, high }

enum PasteMode { flowecho, nativeRestore }

enum PasteSource { flowecho, native }

enum PastePolicyMode { flowechoDefault, nativeDefault }

enum AppScope { allApps, allowList, denyList }

enum TransferState { completed, pendingResume }

class StartPairingRequest {
  StartPairingRequest({
    required this.localDeviceId,
    required this.localAlias,
    required this.peerIp,
  });

  final String localDeviceId;
  final String localAlias;
  final String peerIp;

  Map<String, Object?> toJson() => {
        "local_device_id": localDeviceId,
        "local_alias": localAlias,
        "peer_ip": peerIp,
      };
}

class PairingChallenge {
  PairingChallenge({
    required this.peerIp,
    required this.listenPort,
    required this.otpCode,
    required this.expiresAtMs,
    required this.attemptsRemaining,
  });

  final String peerIp;
  final int listenPort;
  final String otpCode;
  final int expiresAtMs;
  final int attemptsRemaining;

  factory PairingChallenge.fromJson(Map<String, Object?> json) {
    return PairingChallenge(
      peerIp: json["peer_ip"] as String,
      listenPort: json["listen_port"] as int,
      otpCode: json["otp_code"] as String,
      expiresAtMs: json["expires_at_ms"] as int,
      attemptsRemaining: json["attempts_remaining"] as int,
    );
  }
}

class PairDeviceRequest {
  PairDeviceRequest({
    required this.peerIp,
    required this.peerPort,
    required this.otpCode,
    required this.localDeviceId,
    required this.localAlias,
  });

  final String peerIp;
  final int peerPort;
  final String otpCode;
  final String localDeviceId;
  final String localAlias;

  Map<String, Object?> toJson() => {
        "peer_ip": peerIp,
        "peer_port": peerPort,
        "otp_code": otpCode,
        "local_device_id": localDeviceId,
        "local_alias": localAlias,
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

class SendTextRequest {
  SendTextRequest({
    required this.peerIp,
    required this.peerPort,
    required this.text,
  });

  final String peerIp;
  final int peerPort;
  final String text;

  Map<String, Object?> toJson() => {
        "peer_ip": peerIp,
        "peer_port": peerPort,
        "text": text,
      };
}

class SendFileRequest {
  SendFileRequest({
    required this.peerIp,
    required this.peerPort,
    required this.filePath,
  });

  final String peerIp;
  final int peerPort;
  final String filePath;

  Map<String, Object?> toJson() => {
        "peer_ip": peerIp,
        "peer_port": peerPort,
        "file_path": filePath,
      };
}

class ResumeTransferRequest {
  ResumeTransferRequest({
    required this.peerIp,
    required this.peerPort,
    required this.resumeToken,
  });

  final String peerIp;
  final int peerPort;
  final String resumeToken;

  Map<String, Object?> toJson() => {
        "peer_ip": peerIp,
        "peer_port": peerPort,
        "resume_token": resumeToken,
      };
}

class TransferOutcome {
  TransferOutcome({
    required this.sessionId,
    required this.resumeToken,
    required this.state,
    required this.bytesTransferred,
    required this.totalBytes,
    required this.missingChunks,
    required this.message,
  });

  final String sessionId;
  final String resumeToken;
  final TransferState state;
  final int bytesTransferred;
  final int totalBytes;
  final List<int> missingChunks;
  final String message;

  factory TransferOutcome.fromJson(Map<String, Object?> json) {
    return TransferOutcome(
      sessionId: json["session_id"] as String,
      resumeToken: json["resume_token"] as String,
      state: TransferState.values.firstWhere(
        (e) => switch (e) {
          TransferState.completed => "completed",
          TransferState.pendingResume => "pending_resume",
        } == json["state"],
      ),
      bytesTransferred: json["bytes_transferred"] as int,
      totalBytes: json["total_bytes"] as int,
      missingChunks: (json["missing_chunks"] as List<dynamic>)
          .map((item) => item as int)
          .toList(),
      message: json["message"] as String,
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
