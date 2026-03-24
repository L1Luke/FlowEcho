import "package:flowecho_app/src/flowecho_models.dart";
import "package:flowecho_app/src/flowpaste_panel_controller.dart";
import "package:flowecho_app/src/flowpaste_panel_facade.dart";
import "package:flowecho_app/src/flowpaste_panel_state.dart";
import "package:test/test.dart";

PayloadManifest manifest({
  required PayloadType type,
  required int size,
}) {
  return PayloadManifest(
    payloadId: "payload-facade",
    type: type,
    mime: "application/octet-stream",
    size: size,
    hash: "abc",
    createdAt: 1700000000000,
  );
}

void main() {
  test("facade exposes default view model for ui binding", () {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final facade = FlowPastePanelFacade(controller);
    final vm = facade.buildViewModel();
    expect(vm.selectedMode, FlowPasteMode.lowLatency);
    expect(vm.defaultSaveDirectory, "/tmp/flowecho");
    expect(vm.modeOptions.length, 2);
    controller.dispose();
  });

  test("facade updates blocked type and max bytes", () {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final facade = FlowPastePanelFacade(controller);
    facade.onBlockedTypeToggled(PayloadType.file, true);
    facade.onMaxAutoSyncBytesChanged("10");

    final vm = facade.buildViewModel();
    expect(vm.blockedTypes.contains(PayloadType.file), isTrue);
    expect(vm.maxAutoSyncBytes, 10);
    controller.dispose();
  });

  test("facade preview and policy mapping are consistent", () {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowTraffic,
        defaultSaveDirectory: "/Users/luke/Downloads",
        blockedTypes: const {PayloadType.image},
      ),
    );
    final facade = FlowPastePanelFacade(controller);
    final preview = facade.previewFor(
      manifest: manifest(type: PayloadType.text, size: 20),
      fileName: "note.txt",
    );
    expect(preview.shouldAutoPublish, isFalse);
    expect(preview.shouldRequestOnPaste, isTrue);
    expect(preview.resolvedSavePath, "/Users/luke/Downloads/note.txt");
    expect(facade.buildPastePolicy().mode, PastePolicyMode.nativeDefault);
    controller.dispose();
  });
}
