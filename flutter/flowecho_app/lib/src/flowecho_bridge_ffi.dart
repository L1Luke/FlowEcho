import "dart:convert";
import "dart:ffi";

import "package:ffi/ffi.dart";

import "flowecho_bridge_api.dart";
import "flowecho_models.dart";

typedef _NativeHandlerNative = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _NativeHandlerDart = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _NativeFreeNative = Void Function(Pointer<Utf8>);
typedef _NativeFreeDart = void Function(Pointer<Utf8>);

class FlowEchoBridgeFfi implements FlowEchoBridgeApi {
  FlowEchoBridgeFfi(DynamicLibrary lib)
      : _startPairing = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_start_pairing",
        ),
        _pairDevice = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_pair_device",
        ),
        _sendText = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_send_text",
        ),
        _sendFile = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_send_file",
        ),
        _resumeTransfer =
            lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_resume_transfer",
        ),
        _publishClipboard =
            lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_publish_clipboard",
        ),
        _startTransfer = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_start_transfer",
        ),
        _applyPaste = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_apply_paste",
        ),
        _setPastePolicy = lib.lookupFunction<_NativeHandlerNative, _NativeHandlerDart>(
          "flowecho_set_paste_policy",
        ),
        _freeString = lib.lookupFunction<_NativeFreeNative, _NativeFreeDart>(
          "flowecho_free_string",
        );

  final _NativeHandlerDart _startPairing;
  final _NativeHandlerDart _pairDevice;
  final _NativeHandlerDart _sendText;
  final _NativeHandlerDart _sendFile;
  final _NativeHandlerDart _resumeTransfer;
  final _NativeHandlerDart _publishClipboard;
  final _NativeHandlerDart _startTransfer;
  final _NativeHandlerDart _applyPaste;
  final _NativeHandlerDart _setPastePolicy;
  final _NativeFreeDart _freeString;

  @override
  Future<PairingChallenge> startPairing(StartPairingRequest request) async {
    final data = _invoke(_startPairing, request.toJson());
    return PairingChallenge.fromJson(data);
  }

  @override
  Future<DeviceTrust> pairDevice(PairDeviceRequest request) async {
    final data = _invoke(_pairDevice, request.toJson());
    return DeviceTrust.fromJson(data);
  }

  @override
  Future<TransferOutcome> sendText(SendTextRequest request) async {
    final data = _invoke(_sendText, request.toJson());
    return TransferOutcome.fromJson(data);
  }

  @override
  Future<TransferOutcome> sendFile(SendFileRequest request) async {
    final data = _invoke(_sendFile, request.toJson());
    return TransferOutcome.fromJson(data);
  }

  @override
  Future<TransferOutcome> resumeTransfer(ResumeTransferRequest request) async {
    final data = _invoke(_resumeTransfer, request.toJson());
    return TransferOutcome.fromJson(data);
  }

  @override
  Future<SyncAck> publishClipboard(PublishClipboardRequest request) async {
    final data = _invoke(_publishClipboard, request.toJson());
    return SyncAck.fromJson(data);
  }

  @override
  Future<TransferSession> startTransfer(StartTransferRequest request) async {
    final data = _invoke(_startTransfer, request.toJson());
    return TransferSession.fromJson(data);
  }

  @override
  Future<PasteResult> applyPaste(ApplyPasteRequest request) async {
    final data = _invoke(_applyPaste, request.toJson());
    return PasteResult.fromJson(data);
  }

  @override
  Future<SetPastePolicyResult> setPastePolicy(PastePolicy policy) async {
    final data = _invoke(_setPastePolicy, policy.toJson());
    return SetPastePolicyResult.fromJson(data);
  }

  Map<String, Object?> _invoke(
    _NativeHandlerDart handler,
    Map<String, Object?> request,
  ) {
    final reqPtr = jsonEncode(request).toNativeUtf8();
    final respPtr = handler(reqPtr);
    malloc.free(reqPtr);

    final response = respPtr.toDartString();
    _freeString(respPtr);
    final decoded = jsonDecode(response) as Map<String, Object?>;
    final ok = decoded["ok"] as bool? ?? false;
    if (!ok) {
      final error = decoded["error"] as Map<String, Object?>? ?? const {};
      throw FlowEchoRpcError(
        code: error["code"] as int? ?? -1,
        message: error["message"] as String? ?? "unknown rpc error",
      );
    }
    return decoded["data"] as Map<String, Object?>;
  }
}
