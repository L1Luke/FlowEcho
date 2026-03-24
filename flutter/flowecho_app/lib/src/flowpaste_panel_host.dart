import "dart:async";
import "dart:convert";

import "package:flutter/widgets.dart";

import "flowecho_bridge_api.dart";
import "flowecho_models.dart";
import "flowpaste_panel_controller.dart";
import "flowpaste_panel_facade.dart";
import "flowpaste_panel_state.dart";
import "flowpaste_panel_widget.dart";
import "flowpaste_policy_coordinator.dart";

class FlowPastePanelHost extends StatefulWidget {
  const FlowPastePanelHost({
    super.key,
    required this.controller,
    required this.bridge,
    required this.repository,
    required this.previewManifest,
    required this.previewFileName,
    this.onPolicyChanged,
    this.onSyncError,
  });

  final FlowPastePanelController controller;
  final FlowEchoBridgeApi bridge;
  final FlowPastePreferencesRepository repository;
  final PayloadManifest previewManifest;
  final String previewFileName;
  final ValueChanged<PastePolicy>? onPolicyChanged;
  final void Function(Object error, StackTrace stackTrace)? onSyncError;

  @override
  State<FlowPastePanelHost> createState() => _FlowPastePanelHostState();
}

class _FlowPastePanelHostState extends State<FlowPastePanelHost> {
  late final FlowPastePanelFacade _facade;
  late final FlowPastePolicyCoordinator _coordinator;
  late final StreamSubscription<FlowPastePanelViewState> _stateSubscription;

  bool _ready = false;
  String _preferenceFingerprint = "";

  @override
  void initState() {
    super.initState();
    _facade = FlowPastePanelFacade(widget.controller);
    _coordinator = FlowPastePolicyCoordinator(
      controller: widget.controller,
      bridge: widget.bridge,
      repository: widget.repository,
    );
    _stateSubscription = widget.controller.states.listen(_onControllerState);
    unawaited(_initialize());
  }

  @override
  void dispose() {
    unawaited(_stateSubscription.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_ready) {
      return const SizedBox(
        key: Key("flowpaste_host_loading"),
        width: 0,
        height: 0,
      );
    }
    return FlowPastePanelWidget(
      facade: _facade,
      previewManifest: widget.previewManifest,
      previewFileName: widget.previewFileName,
    );
  }

  Future<void> _initialize() async {
    try {
      await _coordinator.initialize();
    } catch (error, stackTrace) {
      widget.onSyncError?.call(error, stackTrace);
    }
    if (!mounted) {
      return;
    }
    setState(() {
      _preferenceFingerprint =
          _fingerprint(widget.controller.state.preferences);
      _ready = true;
    });
  }

  void _onControllerState(FlowPastePanelViewState state) {
    if (!_ready) {
      return;
    }
    final nextFingerprint = _fingerprint(state.preferences);
    if (nextFingerprint == _preferenceFingerprint) {
      return;
    }
    _preferenceFingerprint = nextFingerprint;
    widget.onPolicyChanged?.call(widget.controller.toPastePolicy());
    unawaited(_persistAndSync());
  }

  Future<void> _persistAndSync() async {
    try {
      await _coordinator.persistAndSync();
    } catch (error, stackTrace) {
      widget.onSyncError?.call(error, stackTrace);
    }
  }

  String _fingerprint(FlowPastePreferences preferences) {
    return jsonEncode(preferences.toJson());
  }
}
