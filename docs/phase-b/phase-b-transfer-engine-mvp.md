# Phase B - Rust 传输引擎 MVP

## 阶段目标

1. 在 Rust Core 实现分片传输与哈希校验的最小可用引擎。
2. 支持断点续传：可根据已确认 chunk 索引恢复发送。
3. 在 `StartTransfer` 流程中返回可执行的会话元数据（chunk size / resume token / offset）。
4. 保持隐私优先：传输面只处理局域网直连场景，不引入云中继。

## 接口设计

### 1) TransferPlanner

- `create_plan(payload_id, bytes, chunk_size) -> TransferPlan`
- 输出：
  - `manifest`：文件总体哈希、chunk 总数、chunk size
  - `chunks`：每个 chunk 的索引、偏移、长度、chunk 哈希

### 2) TransferAssembler

- `new(manifest) -> TransferAssembler`
- `next_missing_chunk() -> Option<u32>`
- `apply_chunk(index, data) -> Result<()>`
- `resume_offset() -> u64`
- `missing_chunks() -> Vec<u32>`
- `finish() -> Result<Vec<u8>>`（校验总哈希后产出完整 payload）

### 3) SessionStore（Phase B 内存态）

- `open_session(payload_id, target_device, chunk_size) -> TransferSession`
- `resume_session(session_id) -> Option<TransferSessionState>`
- 当前仅内存实现，持久化存储留在后续阶段。

## 测试清单

1. 规划器一致性测试
   - chunk 总数、偏移、大小正确。
   - 总哈希与 chunk 哈希可复算。
2. 断点续传测试
   - 部分 chunk 应答后，`resume_offset` 与 `missing_chunks` 正确。
   - 重启 assembler（基于已确认索引）后可继续。
3. 完整性测试
   - 全量 chunk 后 `finish()` 成功，且输出字节等于原文。
   - 任意 chunk 被篡改时 `apply_chunk` 或 `finish` 返回哈希错误。
4. 服务层集成测试
   - `start_transfer` 生成会话包含可恢复元信息。
   - 非法 payload/target 输入返回标准错误码。
