import "flowecho_models.dart";

abstract class FlowEchoBridgeApi {
  Future<DeviceTrust> pairDevice(PairDeviceRequest request);

  Future<SyncAck> publishClipboard(PublishClipboardRequest request);

  Future<TransferSession> startTransfer(StartTransferRequest request);

  Future<PasteResult> applyPaste(ApplyPasteRequest request);

  Future<SetPastePolicyResult> setPastePolicy(PastePolicy policy);
}
