import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:flowecho_app/src/flowecho_models.dart";
import "package:flowecho_app/src/flowpaste_panel_controller.dart";
import "package:flowecho_app/src/flowpaste_panel_facade.dart";
import "package:flowecho_app/src/flowpaste_panel_state.dart";
import "package:flowecho_app/src/flowpaste_panel_widget.dart";

PayloadManifest _manifest() {
  return PayloadManifest(
    payloadId: "p-widget",
    type: PayloadType.text,
    mime: "text/plain",
    size: 12,
    hash: "abc",
    createdAt: 1700000000000,
  );
}

void main() {
  testWidgets("widget renders key controls", (tester) async {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final facade = FlowPastePanelFacade(controller);

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowPastePanelWidget(
            facade: facade,
            previewManifest: _manifest(),
            previewFileName: "note.txt",
          ),
        ),
      ),
    );

    expect(find.byKey(const Key("flowpaste_title")), findsOneWidget);
    expect(find.byKey(const Key("mode_dropdown")), findsOneWidget);
    expect(find.byKey(const Key("save_dir_field")), findsOneWidget);
    expect(find.byKey(const Key("max_bytes_field")), findsOneWidget);

    controller.dispose();
  });

  testWidgets("widget updates controller via user input", (tester) async {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final facade = FlowPastePanelFacade(controller);

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowPastePanelWidget(
            facade: facade,
            previewManifest: _manifest(),
            previewFileName: "note.txt",
          ),
        ),
      ),
    );

    await tester.enterText(
      find.byKey(const Key("save_dir_field")),
      "/Users/luke/Desktop",
    );
    await tester.enterText(
      find.byKey(const Key("max_bytes_field")),
      "10",
    );
    await tester.tap(find.byKey(const Key("blocked_type_file")));
    await tester.pump();

    expect(
      controller.state.preferences.defaultSaveDirectory,
      "/Users/luke/Desktop",
    );
    expect(controller.state.preferences.maxAutoSyncBytes, 10);
    expect(
      controller.state.preferences.blockedTypes.contains(PayloadType.file),
      isTrue,
    );

    controller.dispose();
  });

  testWidgets("preview button shows decision text", (tester) async {
    final controller = FlowPastePanelController(
      initialPreferences: FlowPastePreferences(
        mode: FlowPasteMode.lowLatency,
        defaultSaveDirectory: "/tmp/flowecho",
      ),
    );
    final facade = FlowPastePanelFacade(controller);

    await tester.pumpWidget(
      MaterialApp(
        home: Material(
          child: FlowPastePanelWidget(
            facade: facade,
            previewManifest: _manifest(),
            previewFileName: "note.txt",
          ),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key("preview_button")));
    await tester.pump();

    expect(find.byKey(const Key("preview_auto_publish")), findsOneWidget);
    expect(find.textContaining("AutoPublish: true"), findsOneWidget);
    expect(find.textContaining("ResolvedPath: /tmp/flowecho/note.txt"), findsOneWidget);

    controller.dispose();
  });
}
