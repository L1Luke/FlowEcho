# Phase A - 协议与安全基线

## 阶段目标

1. 定义跨端统一协议模型，确保 Rust Core 与 Flutter Bridge 字段一致。
2. 固化 5 个核心接口：
   - `PairDevice(request_qr, verify_code) -> DeviceTrust`
   - `PublishClipboard(payload_manifest) -> SyncAck`
   - `StartTransfer(payload_id, target_device) -> TransferSession`
   - `ApplyPaste(mode=flowecho|native_restore, payload_id?) -> PasteResult`
   - `SetPastePolicy(flowecho_default, bypass_rules, app_scope)`
3. 建立端到端加密握手最小实现（X25519 + HKDF-SHA256 + ChaCha20-Poly1305）。
4. 输出标准错误码体系，供 Rust/Flutter/平台层统一处理。

## 公共接口设计

### 1) PairDevice

- Request:
  - `request_qr: String`
  - `verify_code: String`
- Response (`DeviceTrust`):
  - `device_id: String`
  - `alias: String`
  - `trust_state: trusted|pending|revoked`
  - `session_key_id: String`

### 2) PublishClipboard

- Request:
  - `source_device: String`
  - `payload_manifest: PayloadManifest`
  - `ttl_ms: u64`
  - `priority: normal|high`
- Response (`SyncAck`):
  - `ack_id: String`
  - `accepted: bool`
  - `reason: Option<String>`

### 3) StartTransfer

- Request:
  - `payload_id: String`
  - `target_device: String`
- Response (`TransferSession`):
  - `session_id: String`
  - `chunk_size: u32`
  - `offset: u64`
  - `resume_token: String`
  - `throughput_hint_kbps: u32`

### 4) ApplyPaste

- Request:
  - `mode: flowecho|native_restore`
  - `payload_id: Option<String>`
- Response (`PasteResult`):
  - `applied: bool`
  - `source: flowecho|native`
  - `restored_native_snapshot: bool`
  - `message: String`

### 5) SetPastePolicy

- Request (`PastePolicy`):
  - `mode: flowecho_default|native_default`
  - `bypass_rules: Vec<String>`
  - `app_scope: all_apps|allow_list|deny_list`
- Response:
  - `saved: bool`
  - `effective_at_ms: u64`

## 数据结构

### PayloadManifest

- `payload_id: String`
- `type: text|image|file`
- `mime: String`
- `size: u64`
- `hash: String` (sha256 hex)
- `created_at: u64` (unix ms)

## 平台能力边界（必须写清）

- Windows/macOS 支持全局粘贴接管策略（后续 Phase C 实现键盘拦截与回退）。
- iOS **不支持跨 App 全局粘贴接管**，仅提供 App 内等价入口与流程。

## 测试清单（Phase A）

1. 协议字段一致性测试
   - `PayloadManifest` JSON 字段必须为 `payload_id/type/mime/size/hash/created_at`。
   - 5 个接口的请求/响应 JSON 字段必须稳定且可序列化反序列化。
2. 加密握手测试
   - 双端 X25519 派生出的会话密钥一致。
   - 使用会话密钥进行 AEAD 加解密往返成功。
   - 错误密钥解密必须失败并返回标准错误。
3. 错误码测试
   - 错误码枚举值唯一且稳定。
   - 错误类型到错误码映射正确。
   - FFI 错误输出结构包含 `code/message`。
