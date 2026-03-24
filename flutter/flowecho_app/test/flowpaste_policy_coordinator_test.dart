import "dart:io";

import "package:flowecho_app/src/flowecho_bridge_api.dart";
import "package:flowecho_app/src/flowecho_models.dart";
import "package:flowecho_app/src/flowpaste_panel_controller.dart";
import "package:flowecho_app/src/flowpaste_panel_state.dart";
import "package:flowecho_app/src/flowpaste_policy_coordinator.dart";
import "package:test/test.dart";

class _FakeBridgeApi implements FlowEchoBridgeApi {
  final List<PastePolicy> setPolicyCalls = [];

  @override
  Future<PairingChallenge> startPairing(StartPairingRequest request) async {
    return PairingChallenge(
      peerIp: request.peerIp,
      listenPort: 47000,
      otpCode: "123456",
      expiresAtMs: 1700000060000,
      attemptsRemaining: 5,
    );
  }

  @override
  Future<TransferOutcome> sendText(SendTextRequest request) async {
    return TransferOutcome(
      sessionId: "tx-text-1",
      resumeToken: "resume-text-1",
      state: TransferState.completed,
      bytesTransferred: request.text.length,
      totalBytes: request.text.length,
      missingChunks: const <int>[],
      message: "ok",
    );
  }

  @override
  Future<TransferOutcome> sendFile(SendFileRequest request) async {
    return TransferOutcome(
      sessionId: "tx-file-1",
      resumeToken: "resume-file-1",
      state: TransferState.pendingResume,
      bytesTransferred: 1,
      totalBytes: 2,
      missingChunks: const <int>[1],
      message: "resume",
    );
  }

  @override
  Future<TransferOutcome> resumeTransfer(ResumeTransferRequest request) async {
    return TransferOutcome(
      sessionId: "tx-file-1",
      resumeToken: request.resumeToken,
      state: TransferState.completed,
      bytesTransferred: 2,
      totalBytes: 2,
      missingChunks: const <int>[],
      message: "done",
    );
  }

  @override
  Future<PasteResult> applyPaste(ApplyPasteRequest request) {
    throw UnimplementedError();
  }

  @override
  Future<DeviceTrust> pairDevice(PairDeviceRequest request) {
    throw UnimplementedError();
  }

  @override
  Future<SyncAck> publishClipboard(PublishClipboardRequest request) {
    throw UnimplementedError();
  }

  @override
  Future<SetPastePolicyResult> setPastePolicy(PastePolicy policy) async {
    setPolicyCalls.add(policy);
    return SetPastePolicyResult(saved: true, effectiveAtMs: 1700000000000);
  }

  @override
  Future<TransferSession> startTransfer(StartTransferRequest request) {
    throw UnimplementedError();
  }
}

void main() {
  test("file repository saves and loads preferences", () async {
    final dir = await Directory.systemTemp.createTemp("flowecho-pref-test");
    addTearDown(() => dir.delete(recursive: true));
    final filePath = "${dir.path}/flowpaste_preferences.json";
    final repo = FileFlowPastePreferencesRepository(filePath: filePath);

    final expected = FlowPastePreferences(
      mode: FlowPasteMode.lowTraffic,
      defaultSaveDirectory: "/Users/luke/Desktop",
      blockedTypes: const {PayloadType.file, PayloadType.image},
      maxAutoSyncBytes: 42,
    );
    await repo.save(expected);

    final loaded = await repo.load();
    expect(loaded, isNotNull);
    expect(loaded?.mode, FlowPasteMode.lowTraffic);
    expect(loaded?.defaultSaveDirectory, "/Users/luke/Desktop");
    expect(loaded?.blockedTypes, contains(PayloadType.file));
    expect(loaded?.blockedTypes, contains(PayloadType.image));
    expect(loaded?.maxAutoSyncBytes, 42);
  });

  test("coordinator initializes from persisted preferences and syncs policy",
      () async {
    final dir = await Directory.systemTemp.createTemp("flowecho-coord-init");
    addTearDown(() => dir.delete(recursive: true));
    final repo = FileFlowPastePreferencesRepository(
      filePath: "${dir.path}/flowpaste_preferences.json",
    );
    await repo.save(
      FlowPastePreferences(
        mode: FlowPasteMode.lowTraffic,
        defaultSaveDirectory: "/Users/luke/Desktop",
        blockedTypes: const {PayloadType.file},
      ),
    );

    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final bridge = _FakeBridgeApi();
    final coordinator = FlowPastePolicyCoordinator(
      controller: controller,
      bridge: bridge,
      repository: repo,
    );

    await coordinator.initialize();

    expect(controller.state.preferences.mode, FlowPasteMode.lowTraffic);
    expect(controller.state.preferences.defaultSaveDirectory,
        "/Users/luke/Desktop");
    expect(bridge.setPolicyCalls.length, 1);
    expect(bridge.setPolicyCalls.single.mode, PastePolicyMode.nativeDefault);
    controller.dispose();
  });

  test("coordinator persists and syncs current controller policy", () async {
    final dir = await Directory.systemTemp.createTemp("flowecho-coord-sync");
    addTearDown(() => dir.delete(recursive: true));
    final repo = FileFlowPastePreferencesRepository(
      filePath: "${dir.path}/flowpaste_preferences.json",
    );
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final bridge = _FakeBridgeApi();
    final coordinator = FlowPastePolicyCoordinator(
      controller: controller,
      bridge: bridge,
      repository: repo,
    );

    controller.setMode(FlowPasteMode.lowTraffic);
    controller.setDefaultSaveDirectory("/Users/luke/Documents");
    await coordinator.persistAndSync();

    final loaded = await repo.load();
    expect(loaded?.mode, FlowPasteMode.lowTraffic);
    expect(loaded?.defaultSaveDirectory, "/Users/luke/Documents");
    expect(bridge.setPolicyCalls.last.mode, PastePolicyMode.nativeDefault);
    controller.dispose();
  });
}
