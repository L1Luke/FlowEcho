import "dart:async";

import "flowecho_models.dart";
import "flowpaste_panel_state.dart";

class FlowPasteDecisionPreview {
  FlowPasteDecisionPreview({
    required this.shouldAutoPublish,
    required this.shouldRequestOnPaste,
    required this.resolvedSavePath,
  });

  final bool shouldAutoPublish;
  final bool shouldRequestOnPaste;
  final String resolvedSavePath;
}

class FlowPastePanelViewState {
  FlowPastePanelViewState({
    required this.preferences,
    this.preview,
  });

  final FlowPastePreferences preferences;
  final FlowPasteDecisionPreview? preview;
}

class FlowPastePanelController {
  FlowPastePanelController({
    required FlowPastePreferences initialPreferences,
  })  : _preferences = initialPreferences,
        _state = FlowPastePanelViewState(preferences: initialPreferences);

  FlowPastePreferences _preferences;
  FlowPastePanelViewState _state;
  final StreamController<FlowPastePanelViewState> _streamController =
      StreamController<FlowPastePanelViewState>.broadcast();

  FlowPastePanelViewState get state => _state;
  Stream<FlowPastePanelViewState> get states => _streamController.stream;

  void setMode(FlowPasteMode mode) {
    _preferences = _preferences.copyWith(mode: mode);
    _emit(preview: _state.preview);
  }

  void setDefaultSaveDirectory(String path) {
    _preferences = _preferences.copyWith(defaultSaveDirectory: path);
    _emit(preview: _state.preview);
  }

  void setBlockedTypes(Set<PayloadType> blockedTypes) {
    _preferences = _preferences.copyWith(blockedTypes: blockedTypes);
    _emit(preview: _state.preview);
  }

  void setMaxAutoSyncBytes(int? maxBytes) {
    _preferences = _preferences.copyWith(
      maxAutoSyncBytes: maxBytes,
      clearMaxAutoSyncBytes: maxBytes == null,
    );
    _emit(preview: _state.preview);
  }

  FlowPasteDecisionPreview previewDecision({
    required PayloadManifest manifest,
    required String fileName,
    String? saveAsPath,
  }) {
    final engine = FlowPastePanelState(preferences: _preferences);
    final preview = FlowPasteDecisionPreview(
      shouldAutoPublish: engine.shouldAutoPublish(manifest),
      shouldRequestOnPaste: engine.shouldRequestOnPaste(manifest),
      resolvedSavePath: engine.resolveSavePath(
        fileName: fileName,
        saveAsPath: saveAsPath,
      ),
    );
    _emit(preview: preview);
    return preview;
  }

  PastePolicy toPastePolicy({List<String>? bypassRules, AppScope? appScope}) {
    return PastePolicy(
      mode: _preferences.mode == FlowPasteMode.lowLatency
          ? PastePolicyMode.flowechoDefault
          : PastePolicyMode.nativeDefault,
      bypassRules:
          bypassRules ??
          const ["password_field", "rdp", "terminal_high_risk"],
      appScope: appScope ?? AppScope.allApps,
    );
  }

  void dispose() {
    _streamController.close();
  }

  void _emit({required FlowPasteDecisionPreview? preview}) {
    _state = FlowPastePanelViewState(
      preferences: _preferences,
      preview: preview,
    );
    _streamController.add(_state);
  }
}
