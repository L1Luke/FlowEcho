import "flowecho_models.dart";

abstract class FlowEchoBridgeApi {
  Future<PairingChallenge> startPairing(StartPairingRequest request);

  Future<DeviceTrust> pairDevice(PairDeviceRequest request);

  Future<TransferOutcome> sendText(SendTextRequest request);

  Future<TransferOutcome> sendFile(SendFileRequest request);

  Future<TransferOutcome> resumeTransfer(ResumeTransferRequest request);

  Future<SyncAck> publishClipboard(PublishClipboardRequest request);

  Future<TransferSession> startTransfer(StartTransferRequest request);

  Future<PasteResult> applyPaste(ApplyPasteRequest request);

  Future<SetPastePolicyResult> setPastePolicy(PastePolicy policy);
}
