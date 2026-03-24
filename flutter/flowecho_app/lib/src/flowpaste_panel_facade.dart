import "flowecho_models.dart";
import "flowpaste_panel_controller.dart";
import "flowpaste_panel_state.dart";

class FlowPasteModeOption {
  FlowPasteModeOption({
    required this.mode,
    required this.title,
    required this.description,
  });

  final FlowPasteMode mode;
  final String title;
  final String description;
}

class FlowPastePanelViewModel {
  FlowPastePanelViewModel({
    required this.selectedMode,
    required this.defaultSaveDirectory,
    required this.maxAutoSyncBytes,
    required this.blockedTypes,
    required this.modeOptions,
    required this.preview,
  });

  final FlowPasteMode selectedMode;
  final String defaultSaveDirectory;
  final int? maxAutoSyncBytes;
  final Set<PayloadType> blockedTypes;
  final List<FlowPasteModeOption> modeOptions;
  final FlowPasteDecisionPreview? preview;
}

class FlowPastePanelFacade {
  FlowPastePanelFacade(this._controller);

  final FlowPastePanelController _controller;

  FlowPastePanelViewModel buildViewModel() {
    final prefs = _controller.state.preferences;
    return FlowPastePanelViewModel(
      selectedMode: prefs.mode,
      defaultSaveDirectory: prefs.defaultSaveDirectory,
      maxAutoSyncBytes: prefs.maxAutoSyncBytes,
      blockedTypes: prefs.blockedTypes,
      modeOptions: [
        FlowPasteModeOption(
          mode: FlowPasteMode.lowLatency,
          title: "低延迟",
          description: "符合规则时复制即同步",
        ),
        FlowPasteModeOption(
          mode: FlowPasteMode.lowTraffic,
          title: "低流量",
          description: "粘贴时请求最新内容，降低常驻流量",
        ),
      ],
      preview: _controller.state.preview,
    );
  }

  void onModeChanged(FlowPasteMode mode) {
    _controller.setMode(mode);
  }

  void onDefaultSaveDirectoryChanged(String path) {
    _controller.setDefaultSaveDirectory(path);
  }

  void onMaxAutoSyncBytesChanged(String? input) {
    if (input == null || input.trim().isEmpty) {
      _controller.setMaxAutoSyncBytes(null);
      return;
    }
    final parsed = int.tryParse(input.trim());
    if (parsed != null && parsed >= 0) {
      _controller.setMaxAutoSyncBytes(parsed);
    }
  }

  void onBlockedTypeToggled(PayloadType type, bool blocked) {
    final next = Set<PayloadType>.from(_controller.state.preferences.blockedTypes);
    if (blocked) {
      next.add(type);
    } else {
      next.remove(type);
    }
    _controller.setBlockedTypes(next);
  }

  FlowPasteDecisionPreview previewFor({
    required PayloadManifest manifest,
    required String fileName,
    String? saveAsPath,
  }) {
    return _controller.previewDecision(
      manifest: manifest,
      fileName: fileName,
      saveAsPath: saveAsPath,
    );
  }

  PastePolicy buildPastePolicy() => _controller.toPastePolicy();
}
