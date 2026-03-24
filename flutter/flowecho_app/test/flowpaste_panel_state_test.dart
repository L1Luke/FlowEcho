import "package:flowecho_app/src/flowecho_models.dart";
import "package:flowecho_app/src/flowpaste_panel_state.dart";
import "package:test/test.dart";

PayloadManifest buildManifest({
  required PayloadType type,
  required int size,
}) {
  return PayloadManifest(
    payloadId: "payload-1",
    type: type,
    mime: "application/octet-stream",
    size: size,
    hash: "abc",
    createdAt: 1700000000000,
  );
}

void main() {
  test("low latency mode auto publishes when rule allows", () {
    final state = FlowPastePanelState(
      preferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
        blockedTypes: const {},
        maxAutoSyncBytes: 1024 * 1024,
      ),
    );
    final manifest = buildManifest(type: PayloadType.text, size: 128);
    expect(state.shouldAutoPublish(manifest), isTrue);
    expect(state.shouldRequestOnPaste(manifest), isFalse);
  });

  test("low traffic mode requests on paste and does not auto publish", () {
    final state = FlowPastePanelState(
      preferences: FlowPastePreferences(
        mode: FlowPasteMode.lowTraffic,
        defaultSaveDirectory: "/tmp/flowecho",
        blockedTypes: const {},
      ),
    );
    final manifest = buildManifest(type: PayloadType.image, size: 2048);
    expect(state.shouldAutoPublish(manifest), isFalse);
    expect(state.shouldRequestOnPaste(manifest), isTrue);
  });

  test("blocked type must not auto publish", () {
    final state = FlowPastePanelState(
      preferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
        blockedTypes: const {PayloadType.file},
      ),
    );
    final manifest = buildManifest(type: PayloadType.file, size: 100);
    expect(state.shouldAutoPublish(manifest), isFalse);
    expect(state.shouldRequestOnPaste(manifest), isFalse);
  });

  test("max auto sync size blocks oversized payload", () {
    final state = FlowPastePanelState(
      preferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
        maxAutoSyncBytes: 10,
      ),
    );
    final manifest = buildManifest(type: PayloadType.text, size: 11);
    expect(state.shouldAutoPublish(manifest), isFalse);
  });

  test("resolveSavePath uses default folder", () {
    final state = FlowPastePanelState(
      preferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/Users/luke/Downloads",
      ),
    );
    expect(
      state.resolveSavePath(fileName: "a.txt"),
      "/Users/luke/Downloads/a.txt",
    );
  });

  test("resolveSavePath uses save-as path when provided", () {
    final state = FlowPastePanelState(
      preferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/Users/luke/Downloads",
      ),
    );
    expect(
      state.resolveSavePath(
        fileName: "a.txt",
        saveAsPath: "/Users/luke/Desktop/custom.txt",
      ),
      "/Users/luke/Desktop/custom.txt",
    );
  });
}
