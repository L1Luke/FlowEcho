import "flowecho_models.dart";

enum FlowPasteMode { lowLatency, lowTraffic }

class FlowPastePreferences {
  FlowPastePreferences({
    required this.mode,
    required this.defaultSaveDirectory,
    this.blockedTypes = const <PayloadType>{},
    this.maxAutoSyncBytes,
  });

  final FlowPasteMode mode;
  final String defaultSaveDirectory;
  final Set<PayloadType> blockedTypes;
  final int? maxAutoSyncBytes;

  Map<String, Object?> toJson() {
    return {
      "mode": switch (mode) {
        FlowPasteMode.lowLatency => "low_latency",
        FlowPasteMode.lowTraffic => "low_traffic",
      },
      "default_save_directory": defaultSaveDirectory,
      "blocked_types": blockedTypes.map((e) => e.name).toList()..sort(),
      "max_auto_sync_bytes": maxAutoSyncBytes,
    };
  }

  factory FlowPastePreferences.fromJson(Map<String, Object?> json) {
    final modeRaw = json["mode"] as String? ?? "low_latency";
    final blockedRaw = (json["blocked_types"] as List<dynamic>? ?? const <dynamic>[]);
    return FlowPastePreferences(
      mode: modeRaw == "low_traffic"
          ? FlowPasteMode.lowTraffic
          : FlowPasteMode.lowLatency,
      defaultSaveDirectory:
          (json["default_save_directory"] as String?) ?? "/Users/luke/Downloads",
      blockedTypes: blockedRaw
          .map((item) => item as String)
          .map(
            (name) => PayloadType.values.firstWhere(
              (value) => value.name == name,
              orElse: () => PayloadType.text,
            ),
          )
          .toSet(),
      maxAutoSyncBytes: (json["max_auto_sync_bytes"] as num?)?.toInt(),
    );
  }

  FlowPastePreferences copyWith({
    FlowPasteMode? mode,
    String? defaultSaveDirectory,
    Set<PayloadType>? blockedTypes,
    int? maxAutoSyncBytes,
    bool clearMaxAutoSyncBytes = false,
  }) {
    return FlowPastePreferences(
      mode: mode ?? this.mode,
      defaultSaveDirectory: defaultSaveDirectory ?? this.defaultSaveDirectory,
      blockedTypes: blockedTypes ?? this.blockedTypes,
      maxAutoSyncBytes: clearMaxAutoSyncBytes
          ? null
          : maxAutoSyncBytes ?? this.maxAutoSyncBytes,
    );
  }
}

class FlowPastePanelState {
  FlowPastePanelState({required this.preferences});

  final FlowPastePreferences preferences;

  /// iOS boundary: this logic must only be called from App-internal entry.
  bool shouldAutoPublish(PayloadManifest manifest) {
    if (_isBlocked(manifest)) {
      return false;
    }
    return preferences.mode == FlowPasteMode.lowLatency;
  }

  bool shouldRequestOnPaste(PayloadManifest manifest) {
    if (_isBlocked(manifest)) {
      return false;
    }
    return preferences.mode == FlowPasteMode.lowTraffic;
  }

  String resolveSavePath({
    required String fileName,
    String? saveAsPath,
  }) {
    if (saveAsPath != null && saveAsPath.trim().isNotEmpty) {
      return saveAsPath;
    }
    final base = preferences.defaultSaveDirectory;
    if (base.endsWith("/") || base.endsWith("\\")) {
      return "$base$fileName";
    }
    return "$base/$fileName";
  }

  bool _isBlocked(PayloadManifest manifest) {
    if (preferences.blockedTypes.contains(manifest.type)) {
      return true;
    }
    final maxBytes = preferences.maxAutoSyncBytes;
    if (maxBytes != null && manifest.size > maxBytes) {
      return true;
    }
    return false;
  }
}
