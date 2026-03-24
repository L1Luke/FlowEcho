import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:flowecho_app/src/flowecho_bridge_api.dart";
import "package:flowecho_app/src/flowecho_lan_panel.dart";
import "package:flowecho_app/src/flowecho_models.dart";

class _FakeBridgeApi implements FlowEchoBridgeApi {
  final List<StartPairingRequest> startPairingCalls = <StartPairingRequest>[];
  final List<PairDeviceRequest> pairDeviceCalls = <PairDeviceRequest>[];
  final List<SendTextRequest> sendTextCalls = <SendTextRequest>[];
  final List<SendFileRequest> sendFileCalls = <SendFileRequest>[];
  final List<ResumeTransferRequest> resumeTransferCalls = <ResumeTransferRequest>[];

  @override
  Future<PairingChallenge> startPairing(StartPairingRequest request) async {
    startPairingCalls.add(request);
    return PairingChallenge(
      peerIp: request.peerIp,
      listenPort: 47000,
      otpCode: "123456",
      expiresAtMs: 1700000060000,
      attemptsRemaining: 5,
    );
  }

  @override
  Future<DeviceTrust> pairDevice(PairDeviceRequest request) async {
    pairDeviceCalls.add(request);
    return DeviceTrust(
      deviceId: "peer-device",
      alias: "Peer Mac",
      trustState: TrustState.trusted,
      sessionKeyId: "session-key-1",
    );
  }

  @override
  Future<TransferOutcome> sendText(SendTextRequest request) async {
    sendTextCalls.add(request);
    return TransferOutcome(
      sessionId: "tx-text-1",
      resumeToken: "resume-text-1",
      state: TransferState.completed,
      bytesTransferred: request.text.length,
      totalBytes: request.text.length,
      missingChunks: const <int>[],
      message: "text delivered",
    );
  }

  @override
  Future<TransferOutcome> sendFile(SendFileRequest request) async {
    sendFileCalls.add(request);
    return TransferOutcome(
      sessionId: "tx-file-1",
      resumeToken: "resume-file-1",
      state: TransferState.pendingResume,
      bytesTransferred: 4096,
      totalBytes: 8192,
      missingChunks: const <int>[1, 3],
      message: "resume required",
    );
  }

  @override
  Future<TransferOutcome> resumeTransfer(ResumeTransferRequest request) async {
    resumeTransferCalls.add(request);
    return TransferOutcome(
      sessionId: "tx-file-1",
      resumeToken: request.resumeToken,
      state: TransferState.completed,
      bytesTransferred: 8192,
      totalBytes: 8192,
      missingChunks: const <int>[],
      message: "transfer complete",
    );
  }

  @override
  Future<SyncAck> publishClipboard(PublishClipboardRequest request) {
    throw UnimplementedError();
  }

  @override
  Future<TransferSession> startTransfer(StartTransferRequest request) {
    throw UnimplementedError();
  }

  @override
  Future<PasteResult> applyPaste(ApplyPasteRequest request) {
    throw UnimplementedError();
  }

  @override
  Future<SetPastePolicyResult> setPastePolicy(PastePolicy policy) {
    throw UnimplementedError();
  }
}

void main() {
  testWidgets("lan panel starts pairing and shows otp challenge", (tester) async {
    final bridge = _FakeBridgeApi();

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowEchoLanPanel(bridge: bridge),
        ),
      ),
    );

    await tester.enterText(
      find.byKey(const Key("pair_local_device_id_field")),
      "ios-device",
    );
    await tester.enterText(
      find.byKey(const Key("pair_local_alias_field")),
      "iPhone",
    );
    await tester.enterText(
      find.byKey(const Key("pair_peer_ip_field")),
      "192.168.31.20",
    );
    await tester.tap(find.byKey(const Key("start_pairing_button")));
    await tester.pump();

    expect(bridge.startPairingCalls, hasLength(1));
    expect(bridge.startPairingCalls.single.localDeviceId, "ios-device");
    expect(find.textContaining("otp=123456"), findsOneWidget);
    expect(find.textContaining("port=47000"), findsOneWidget);
  });

  testWidgets("lan panel pairs and sends text with progress", (tester) async {
    final bridge = _FakeBridgeApi();

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowEchoLanPanel(bridge: bridge),
        ),
      ),
    );

    await tester.enterText(
      find.byKey(const Key("pair_local_device_id_field")),
      "ios-device",
    );
    await tester.enterText(
      find.byKey(const Key("pair_local_alias_field")),
      "iPhone",
    );
    await tester.enterText(
      find.byKey(const Key("pair_peer_ip_field")),
      "192.168.31.20",
    );
    await tester.enterText(
      find.byKey(const Key("pair_peer_port_field")),
      "47000",
    );
    await tester.enterText(
      find.byKey(const Key("pair_otp_field")),
      "123456",
    );
    await tester.tap(find.byKey(const Key("pair_device_button")));
    await tester.pump();

    await tester.enterText(
      find.byKey(const Key("send_text_field")),
      "hello lan",
    );
    await tester.ensureVisible(find.byKey(const Key("send_text_button")));
    await tester.tap(find.byKey(const Key("send_text_button")));
    await tester.pump();

    expect(bridge.pairDeviceCalls, hasLength(1));
    expect(bridge.sendTextCalls, hasLength(1));
    expect(bridge.sendTextCalls.single.text, "hello lan");
    expect(find.textContaining("text delivered"), findsOneWidget);
    expect(find.textContaining("9/9"), findsOneWidget);
  });

  testWidgets("lan panel sends file and resumes pending transfer", (tester) async {
    final bridge = _FakeBridgeApi();

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowEchoLanPanel(bridge: bridge),
        ),
      ),
    );

    await tester.enterText(
      find.byKey(const Key("pair_peer_ip_field")),
      "192.168.31.20",
    );
    await tester.enterText(
      find.byKey(const Key("pair_peer_port_field")),
      "47000",
    );
    await tester.enterText(
      find.byKey(const Key("send_file_path_field")),
      "/tmp/demo.bin",
    );
    await tester.ensureVisible(find.byKey(const Key("send_file_button")));
    await tester.tap(find.byKey(const Key("send_file_button")));
    await tester.pump();

    expect(bridge.sendFileCalls, hasLength(1));
    expect(find.textContaining("resume=resume-file-1"), findsOneWidget);
    expect(find.textContaining("1,3"), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key("resume_token_field")),
      "resume-file-1",
    );
    await tester.ensureVisible(find.byKey(const Key("resume_transfer_button")));
    await tester.tap(find.byKey(const Key("resume_transfer_button")));
    await tester.pump();

    expect(bridge.resumeTransferCalls, hasLength(1));
    expect(find.textContaining("transfer complete"), findsOneWidget);
    expect(find.textContaining("8192/8192"), findsOneWidget);
  });
}
