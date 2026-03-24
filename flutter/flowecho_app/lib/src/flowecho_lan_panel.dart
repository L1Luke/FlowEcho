import "package:flutter/material.dart";

import "flowecho_bridge_api.dart";
import "flowecho_models.dart";

class FlowEchoLanPanel extends StatefulWidget {
  const FlowEchoLanPanel({
    super.key,
    required this.bridge,
  });

  final FlowEchoBridgeApi bridge;

  @override
  State<FlowEchoLanPanel> createState() => _FlowEchoLanPanelState();
}

class _FlowEchoLanPanelState extends State<FlowEchoLanPanel> {
  late final TextEditingController _localDeviceIdController;
  late final TextEditingController _localAliasController;
  late final TextEditingController _peerIpController;
  late final TextEditingController _peerPortController;
  late final TextEditingController _otpController;
  late final TextEditingController _textController;
  late final TextEditingController _filePathController;
  late final TextEditingController _resumeTokenController;

  PairingChallenge? _challenge;
  DeviceTrust? _trustedDevice;
  TransferOutcome? _transferOutcome;
  String _pairingStatus = "未开始配对";
  String _transferStatus = "未开始传输";

  @override
  void initState() {
    super.initState();
    _localDeviceIdController = TextEditingController();
    _localAliasController = TextEditingController();
    _peerIpController = TextEditingController();
    _peerPortController = TextEditingController();
    _otpController = TextEditingController();
    _textController = TextEditingController();
    _filePathController = TextEditingController();
    _resumeTokenController = TextEditingController();
  }

  @override
  void dispose() {
    _localDeviceIdController.dispose();
    _localAliasController.dispose();
    _peerIpController.dispose();
    _peerPortController.dispose();
    _otpController.dispose();
    _textController.dispose();
    _filePathController.dispose();
    _resumeTokenController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            "实网配对与实传（TCP）",
            key: Key("flowecho_lan_title"),
          ),
          const SizedBox(height: 8),
          const Text(
            "iOS 仅保留 App 内入口，不接管跨 App 全局粘贴。",
            key: Key("ios_lan_boundary_note"),
          ),
          const SizedBox(height: 12),
          TextField(
            key: const Key("pair_local_device_id_field"),
            controller: _localDeviceIdController,
            decoration: const InputDecoration(labelText: "本机设备 ID"),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key("pair_local_alias_field"),
            controller: _localAliasController,
            decoration: const InputDecoration(labelText: "本机别名"),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key("pair_peer_ip_field"),
            controller: _peerIpController,
            decoration: const InputDecoration(labelText: "对端 IP"),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key("pair_peer_port_field"),
            controller: _peerPortController,
            decoration: const InputDecoration(labelText: "对端端口"),
            keyboardType: TextInputType.number,
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key("pair_otp_field"),
            controller: _otpController,
            decoration: const InputDecoration(labelText: "6 位 OTP"),
            keyboardType: TextInputType.number,
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              ElevatedButton(
                key: const Key("start_pairing_button"),
                onPressed: _startPairing,
                child: const Text("开始配对"),
              ),
              ElevatedButton(
                key: const Key("pair_device_button"),
                onPressed: _pairDevice,
                child: const Text("确认配对"),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            _buildPairingSummary(),
            key: const Key("pairing_status_text"),
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key("send_text_field"),
            controller: _textController,
            decoration: const InputDecoration(labelText: "发送文本"),
            minLines: 1,
            maxLines: 3,
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key("send_file_path_field"),
            controller: _filePathController,
            decoration: const InputDecoration(labelText: "发送文件路径"),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key("resume_token_field"),
            controller: _resumeTokenController,
            decoration: const InputDecoration(labelText: "恢复令牌"),
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              ElevatedButton(
                key: const Key("send_text_button"),
                onPressed: _sendText,
                child: const Text("发送文本"),
              ),
              ElevatedButton(
                key: const Key("send_file_button"),
                onPressed: _sendFile,
                child: const Text("发送文件"),
              ),
              ElevatedButton(
                key: const Key("resume_transfer_button"),
                onPressed: _resumeTransfer,
                child: const Text("恢复传输"),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            _buildTransferSummary(),
            key: const Key("transfer_status_text"),
          ),
        ],
      ),
    );
  }

  Future<void> _startPairing() async {
    try {
      final challenge = await widget.bridge.startPairing(
        StartPairingRequest(
          localDeviceId: _localDeviceIdController.text.trim(),
          localAlias: _localAliasController.text.trim(),
          peerIp: _peerIpController.text.trim(),
        ),
      );
      setState(() {
        _challenge = challenge;
        _otpController.text = challenge.otpCode;
        _peerPortController.text = challenge.listenPort.toString();
        _pairingStatus = "OTP 已签发";
      });
    } catch (error) {
      setState(() {
        _pairingStatus = "配对失败: $error";
      });
    }
  }

  Future<void> _pairDevice() async {
    try {
      final trustedDevice = await widget.bridge.pairDevice(
        PairDeviceRequest(
          peerIp: _peerIpController.text.trim(),
          peerPort: _parsePort(_peerPortController.text),
          otpCode: _otpController.text.trim(),
          localDeviceId: _localDeviceIdController.text.trim(),
          localAlias: _localAliasController.text.trim(),
        ),
      );
      setState(() {
        _trustedDevice = trustedDevice;
        _pairingStatus = "设备已信任";
      });
    } catch (error) {
      setState(() {
        _pairingStatus = "确认失败: $error";
      });
    }
  }

  Future<void> _sendText() async {
    await _runTransfer(
      () => widget.bridge.sendText(
        SendTextRequest(
          peerIp: _peerIpController.text.trim(),
          peerPort: _parsePort(_peerPortController.text),
          text: _textController.text,
        ),
      ),
    );
  }

  Future<void> _sendFile() async {
    await _runTransfer(
      () => widget.bridge.sendFile(
        SendFileRequest(
          peerIp: _peerIpController.text.trim(),
          peerPort: _parsePort(_peerPortController.text),
          filePath: _filePathController.text.trim(),
        ),
      ),
    );
  }

  Future<void> _resumeTransfer() async {
    await _runTransfer(
      () => widget.bridge.resumeTransfer(
        ResumeTransferRequest(
          peerIp: _peerIpController.text.trim(),
          peerPort: _parsePort(_peerPortController.text),
          resumeToken: _resumeTokenController.text.trim(),
        ),
      ),
    );
  }

  Future<void> _runTransfer(Future<TransferOutcome> Function() action) async {
    try {
      final outcome = await action();
      setState(() {
        _transferOutcome = outcome;
        if (_resumeTokenController.text.trim().isEmpty) {
          _resumeTokenController.text = outcome.resumeToken;
        }
        _transferStatus = outcome.message;
      });
    } catch (error) {
      setState(() {
        _transferStatus = "传输失败: $error";
      });
    }
  }

  String _buildPairingSummary() {
    final challenge = _challenge;
    final trustedDevice = _trustedDevice;
    final challengeText = challenge == null
        ? "challenge=none"
        : "otp=${challenge.otpCode}, port=${challenge.listenPort}, expires=${challenge.expiresAtMs}, attempts=${challenge.attemptsRemaining}";
    final trustedText = trustedDevice == null
        ? "trusted=none"
        : "trusted=${trustedDevice.alias}/${trustedDevice.deviceId}";
    return "$_pairingStatus | $challengeText | $trustedText";
  }

  String _buildTransferSummary() {
    final outcome = _transferOutcome;
    if (outcome == null) {
      return "$_transferStatus | progress=0/0 | resume=none | missing=none";
    }
    final missingText = outcome.missingChunks.isEmpty
        ? "none"
        : outcome.missingChunks.join(",");
    final stateText = outcome.state == TransferState.completed
        ? "completed"
        : "pending_resume";
    return "$_transferStatus | state=$stateText | progress=${outcome.bytesTransferred}/${outcome.totalBytes} | resume=${outcome.resumeToken} | missing=$missingText";
  }

  int _parsePort(String rawPort) {
    return int.tryParse(rawPort.trim()) ?? 0;
  }
}
