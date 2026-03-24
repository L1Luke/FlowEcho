import "dart:convert";
import "dart:io";

import "flowecho_bridge_api.dart";
import "flowpaste_panel_controller.dart";
import "flowpaste_panel_state.dart";

abstract class FlowPastePreferencesRepository {
  Future<FlowPastePreferences?> load();

  Future<void> save(FlowPastePreferences preferences);
}

class FileFlowPastePreferencesRepository
    implements FlowPastePreferencesRepository {
  FileFlowPastePreferencesRepository({
    required this.filePath,
  });

  final String filePath;

  @override
  Future<FlowPastePreferences?> load() async {
    final file = File(filePath);
    if (!await file.exists()) {
      return null;
    }

    try {
      final raw = await file.readAsString();
      final decoded = jsonDecode(raw);
      if (decoded is! Map<String, dynamic>) {
        return null;
      }
      return FlowPastePreferences.fromJson(decoded);
    } on FormatException {
      return null;
    }
  }

  @override
  Future<void> save(FlowPastePreferences preferences) async {
    final file = File(filePath);
    await file.parent.create(recursive: true);
    final payload = jsonEncode(preferences.toJson());
    await file.writeAsString(payload, flush: true);
  }
}

class FlowPastePolicyCoordinator {
  FlowPastePolicyCoordinator({
    required FlowPastePanelController controller,
    required FlowEchoBridgeApi bridge,
    required FlowPastePreferencesRepository repository,
  })  : _controller = controller,
        _bridge = bridge,
        _repository = repository;

  final FlowPastePanelController _controller;
  final FlowEchoBridgeApi _bridge;
  final FlowPastePreferencesRepository _repository;

  Future<void> initialize() async {
    final persisted = await _repository.load();
    if (persisted != null) {
      _controller.replacePreferences(persisted);
    }
    await _bridge.setPastePolicy(_controller.toPastePolicy());
  }

  Future<void> persistAndSync() async {
    await _repository.save(_controller.state.preferences);
    await _bridge.setPastePolicy(_controller.toPastePolicy());
  }
}
