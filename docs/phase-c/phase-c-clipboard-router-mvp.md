# Phase C - 剪贴板路由 MVP

## 阶段目标

1. 落地桌面端粘贴分流策略：
   - 默认 `Cmd/Ctrl+V` 优先走 FlowEcho。
   - `Cmd/Ctrl+Shift+V` 强制回退原生剪贴板。
2. 落地高风险场景旁路：
   - 密码框、远程桌面、终端高风险会话默认旁路到原生剪贴板。
3. 明确 iOS 能力边界：
   - iOS 仅支持 App 内等价入口，不做跨 App 全局接管。

## 接口设计

### 1) PasteRouteContext

- `platform: windows|macos|ios`
- `hotkey: default_paste|native_fallback_paste`
- `is_password_field: bool`
- `is_remote_session: bool`
- `is_terminal_session: bool`
- `is_in_app_entry: bool`（iOS 必填语义）
- `app_in_scope: bool`（策略作用域判定结果）

### 2) PasteRouter

- `decide(mode, policy, context) -> RouteDecision`
- `RouteDecision`:
  - `source: flowecho|native`
  - `restored_native_snapshot: bool`
  - `reason: String`

## 测试清单

1. 默认接管测试
   - 桌面 `default_paste` + FlowEcho 默认策略 -> `source=flowecho`
2. 原生回退测试
   - 桌面 `native_fallback_paste` -> `source=native` 且 `restored_native_snapshot=true`
3. 高风险旁路测试
   - 密码框/远程桌面/终端高风险 -> `source=native`
4. iOS 能力边界测试
   - iOS `is_in_app_entry=false` -> `source=native`（不允许全局接管）
   - iOS `is_in_app_entry=true` + 合法策略 -> 可 `source=flowecho`
5. 服务层集成测试
   - `apply_paste` 在 context 下输出与 Router 判定一致。
