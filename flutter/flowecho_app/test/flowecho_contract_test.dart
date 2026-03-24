import "package:flowecho_app/src/flowecho_models.dart";
import "package:test/test.dart";

void main() {
  test("payload manifest uses rust contract keys", () {
    final manifest = PayloadManifest(
      payloadId: "payload-1",
      type: PayloadType.file,
      mime: "application/octet-stream",
      size: 2048,
      hash: "abc",
      createdAt: 1700000000000,
    );

    final json = manifest.toJson();
    expect(json["payload_id"], "payload-1");
    expect(json["type"], "file");
    expect(json["mime"], "application/octet-stream");
    expect(json["size"], 2048);
    expect(json["hash"], "abc");
    expect(json["created_at"], 1700000000000);
  });

  test("apply paste mode maps to flowecho or native_restore", () {
    final flowechoJson = ApplyPasteRequest(mode: PasteMode.flowecho).toJson();
    final nativeJson = ApplyPasteRequest(mode: PasteMode.nativeRestore).toJson();
    expect(flowechoJson["mode"], "flowecho");
    expect(nativeJson["mode"], "native_restore");
  });

  test("paste policy maps to stable wire values", () {
    final policy = PastePolicy(
      mode: PastePolicyMode.flowechoDefault,
      bypassRules: const ["password_field", "rdp"],
      appScope: AppScope.allowList,
    );

    final json = policy.toJson();
    expect(json["mode"], "flowecho_default");
    expect(json["app_scope"], "allow_list");
    expect(json["bypass_rules"], ["password_field", "rdp"]);
  });
}
