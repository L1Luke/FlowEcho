import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:flowecho_app/src/flowecho_bridge_api.dart";
import "package:flowecho_app/src/flowecho_models.dart";
import "package:flowecho_app/src/flowpaste_panel_controller.dart";
import "package:flowecho_app/src/flowpaste_panel_host.dart";
import "package:flowecho_app/src/flowpaste_panel_state.dart";
import "package:flowecho_app/src/flowpaste_policy_coordinator.dart";

class _FakeBridgeApi implements FlowEchoBridgeApi {
  final List<PastePolicy> setPolicyCalls = <PastePolicy>[];

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

class _MemoryPreferencesRepository implements FlowPastePreferencesRepository {
  _MemoryPreferencesRepository(this._value);

  FlowPastePreferences? _value;

  @override
  Future<FlowPastePreferences?> load() async => _value;

  @override
  Future<void> save(FlowPastePreferences preferences) async {
    _value = preferences;
  }
}

PayloadManifest _manifest() {
  return PayloadManifest(
    payloadId: "payload-host",
    type: PayloadType.text,
    mime: "text/plain",
    size: 8,
    hash: "abc",
    createdAt: 1700000000000,
  );
}

void main() {
  testWidgets("host initializes from repository and syncs policy", (
    tester,
  ) async {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final bridge = _FakeBridgeApi();
    final repository = _MemoryPreferencesRepository(
      FlowPastePreferences(
        mode: FlowPasteMode.lowTraffic,
        defaultSaveDirectory: "/Users/luke/Desktop",
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowPastePanelHost(
            controller: controller,
            bridge: bridge,
            repository: repository,
            previewManifest: _manifest(),
            previewFileName: "note.txt",
          ),
        ),
      ),
    );
    await tester.pump();

    expect(controller.state.preferences.mode, FlowPasteMode.lowTraffic);
    expect(
      controller.state.preferences.defaultSaveDirectory,
      "/Users/luke/Desktop",
    );
    expect(bridge.setPolicyCalls.length, 1);
    expect(bridge.setPolicyCalls.single.mode, PastePolicyMode.nativeDefault);
    expect(find.byKey(const Key("flowpaste_title")), findsOneWidget);
    expect(find.byKey(const Key("flowecho_lan_title")), findsOneWidget);

    controller.dispose();
  });

  testWidgets("host persists and syncs policy on user updates", (tester) async {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final bridge = _FakeBridgeApi();
    final repository = _MemoryPreferencesRepository(null);

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowPastePanelHost(
            controller: controller,
            bridge: bridge,
            repository: repository,
            previewManifest: _manifest(),
            previewFileName: "note.txt",
          ),
        ),
      ),
    );
    await tester.pump();

    await tester.enterText(
      find.byKey(const Key("save_dir_field")),
      "/Users/luke/Documents",
    );
    await tester.pump();

    expect(repository._value, isNotNull);
    expect(repository._value?.defaultSaveDirectory, "/Users/luke/Documents");
    expect(bridge.setPolicyCalls.length, greaterThanOrEqualTo(2));

    controller.dispose();
  });
}
