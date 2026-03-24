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
