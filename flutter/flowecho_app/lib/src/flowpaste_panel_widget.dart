import "package:flutter/material.dart";

import "flowecho_models.dart";
import "flowpaste_panel_facade.dart";
import "flowpaste_panel_state.dart";

class FlowPastePanelWidget extends StatefulWidget {
  const FlowPastePanelWidget({
    super.key,
    required this.facade,
    required this.previewManifest,
    required this.previewFileName,
    this.onPolicyChanged,
  });

  final FlowPastePanelFacade facade;
  final PayloadManifest previewManifest;
  final String previewFileName;
  final ValueChanged<PastePolicy>? onPolicyChanged;

  @override
  State<FlowPastePanelWidget> createState() => _FlowPastePanelWidgetState();
}

class _FlowPastePanelWidgetState extends State<FlowPastePanelWidget> {
  late FlowPastePanelViewModel _vm;
  late TextEditingController _dirController;
  late TextEditingController _maxBytesController;

  @override
  void initState() {
    super.initState();
    _vm = widget.facade.buildViewModel();
    _dirController = TextEditingController(text: _vm.defaultSaveDirectory);
    _maxBytesController =
        TextEditingController(text: _vm.maxAutoSyncBytes?.toString() ?? "");
  }

  @override
  void dispose() {
    _dirController.dispose();
    _maxBytesController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          "FlowPaste 面板（MVP）",
          key: Key("flowpaste_title"),
        ),
        const SizedBox(height: 8),
        const Text(
          "iOS 仅 App 内等价入口，不支持跨 App 全局接管。",
          key: Key("ios_boundary_note"),
        ),
        const SizedBox(height: 12),
        DropdownButton<FlowPasteMode>(
          key: const Key("mode_dropdown"),
          value: _vm.selectedMode,
          items: _vm.modeOptions
              .map(
                (option) => DropdownMenuItem<FlowPasteMode>(
                  value: option.mode,
                  child: Text(option.title),
                ),
              )
              .toList(),
          onChanged: (mode) {
            if (mode == null) return;
            widget.facade.onModeChanged(mode);
            _refresh();
          },
        ),
        const SizedBox(height: 12),
        TextField(
          key: const Key("save_dir_field"),
          controller: _dirController,
          decoration: const InputDecoration(labelText: "默认保存目录"),
          onChanged: (value) {
            widget.facade.onDefaultSaveDirectoryChanged(value);
            _refresh();
          },
        ),
        const SizedBox(height: 12),
        TextField(
          key: const Key("max_bytes_field"),
          controller: _maxBytesController,
          decoration: const InputDecoration(labelText: "自动同步上限（字节，可空）"),
          keyboardType: TextInputType.number,
          onChanged: (value) {
            widget.facade.onMaxAutoSyncBytesChanged(value);
            _refresh();
          },
        ),
        const SizedBox(height: 12),
        ...PayloadType.values.map(
          (type) => CheckboxListTile(
            key: Key("blocked_type_${type.name}"),
            contentPadding: EdgeInsets.zero,
            title: Text("屏蔽类型: ${type.name}"),
            value: _vm.blockedTypes.contains(type),
            onChanged: (blocked) {
              widget.facade.onBlockedTypeToggled(type, blocked ?? false);
              _refresh();
            },
          ),
        ),
        const SizedBox(height: 12),
        ElevatedButton(
          key: const Key("preview_button"),
          onPressed: () {
            widget.facade.previewFor(
              manifest: widget.previewManifest,
              fileName: widget.previewFileName,
            );
            _refresh();
          },
          child: const Text("预览决策"),
        ),
        const SizedBox(height: 8),
        Text(
          "AutoPublish: ${_vm.preview?.shouldAutoPublish ?? false}",
          key: const Key("preview_auto_publish"),
        ),
        Text(
          "RequestOnPaste: ${_vm.preview?.shouldRequestOnPaste ?? false}",
          key: const Key("preview_request_on_paste"),
        ),
        Text(
          "ResolvedPath: ${_vm.preview?.resolvedSavePath ?? '-'}",
          key: const Key("preview_resolved_path"),
        ),
      ],
    );
  }

  void _refresh() {
    setState(() {
      _vm = widget.facade.buildViewModel();
    });
    widget.onPolicyChanged?.call(widget.facade.buildPastePolicy());
  }
}
