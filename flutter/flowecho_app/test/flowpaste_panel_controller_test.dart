import "package:flowecho_app/src/flowecho_models.dart";
import "package:flowecho_app/src/flowpaste_panel_controller.dart";
import "package:flowecho_app/src/flowpaste_panel_state.dart";
import "package:test/test.dart";

PayloadManifest buildManifest({
  required PayloadType type,
  required int size,
}) {
  return PayloadManifest(
    payloadId: "payload-ctrl",
    type: type,
    mime: "application/octet-stream",
    size: size,
    hash: "abc",
    createdAt: 1700000001000,
  );
}

void main() {
  test("controller updates mode and maps to paste policy", () {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    controller.setMode(FlowPasteMode.lowTraffic);
    expect(controller.state.preferences.mode, FlowPasteMode.lowTraffic);
    expect(controller.toPastePolicy().mode, PastePolicyMode.nativeDefault);
    controller.dispose();
  });

  test("controller preview reflects blocked rules and path resolution", () {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/Users/luke/Downloads",
        blockedTypes: const {PayloadType.file},
      ),
    );
    final preview = controller.previewDecision(
      manifest: buildManifest(type: PayloadType.file, size: 123),
      fileName: "a.txt",
    );
    expect(preview.shouldAutoPublish, isFalse);
    expect(preview.shouldRequestOnPaste, isFalse);
    expect(preview.resolvedSavePath, "/Users/luke/Downloads/a.txt");
    controller.dispose();
  });

  test("controller emits state updates for ui binding", () async {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );

    final states = <FlowPastePanelViewState>[];
    final sub = controller.states.listen(states.add);

    controller.setDefaultSaveDirectory("/Users/luke/Desktop");
    controller.setMaxAutoSyncBytes(10);
    controller.previewDecision(
      manifest: buildManifest(type: PayloadType.text, size: 11),
      fileName: "b.txt",
      saveAsPath: "/Users/luke/Desktop/custom.txt",
    );

    await Future<void>.delayed(const Duration(milliseconds: 10));
    expect(states.length, greaterThanOrEqualTo(3));
    expect(
      states.last.preferences.defaultSaveDirectory,
      "/Users/luke/Desktop",
    );
    expect(states.last.preview?.shouldAutoPublish, isFalse);
    expect(
      states.last.preview?.resolvedSavePath,
      "/Users/luke/Desktop/custom.txt",
    );

    await sub.cancel();
    controller.dispose();
  });

  test("controller can replace full preferences snapshot", () {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );

    controller.replacePreferences(
      FlowPastePreferences(
        mode: FlowPasteMode.lowTraffic,
        defaultSaveDirectory: "/Users/luke/Desktop",
        blockedTypes: const {PayloadType.file},
        maxAutoSyncBytes: 42,
      ),
    );

    expect(controller.state.preferences.mode, FlowPasteMode.lowTraffic);
    expect(
      controller.state.preferences.defaultSaveDirectory,
      "/Users/luke/Desktop",
    );
    expect(controller.state.preferences.blockedTypes, {PayloadType.file});
    expect(controller.state.preferences.maxAutoSyncBytes, 42);
    controller.dispose();
  });
}
