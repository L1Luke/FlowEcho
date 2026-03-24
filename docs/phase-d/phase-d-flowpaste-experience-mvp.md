# Phase D - FlowPaste 体验与策略 MVP

## 阶段目标

1. 提供 FlowPaste 面板状态模型，支撑策略切换与粘贴/保存决策。
2. 支持默认保存目录与“另存为”路径决策。
3. 支持规则过滤（按类型、按大小）控制是否自动同步。
4. 支持“低延迟/低流量”模式切换。

## 接口设计

### 1) FlowPastePreferences

- `mode: low_latency | low_traffic`
- `default_save_directory: String`
- `blocked_types: Set<PayloadType>`
- `max_auto_sync_bytes: int?`

### 2) FlowPastePanelState

- `preferences: FlowPastePreferences`
- `shouldAutoPublish(PayloadManifest) -> bool`
- `shouldRequestOnPaste(PayloadManifest) -> bool`
- `resolveSavePath(file_name, save_as_path?) -> String`

## 测试清单

1. 模式切换
   - `low_latency`：符合规则时可自动同步。
   - `low_traffic`：不自动同步，改为粘贴时请求。
2. 规则过滤
   - 被屏蔽类型必须不自动同步。
   - 超过阈值 `max_auto_sync_bytes` 必须不自动同步。
3. 保存路径
   - 默认保存动作拼接默认目录。
   - 另存为动作优先使用 `save_as_path`。
4. 平台边界说明
   - iOS 仅在 App 内入口执行上述体验策略，不承诺跨 App 全局接管。
