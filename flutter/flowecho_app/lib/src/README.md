# flutter/flowecho_app/lib/src

## 职责

实现 Flutter 侧桥接模型、FlowPaste 逻辑与宿主组件。

## 关键文件

- `flowecho_models.dart`: 跨层数据模型
- `flowecho_bridge_api.dart`: Bridge 抽象接口
- `flowecho_bridge_ffi.dart`: Rust FFI 实现
- `flowpaste_panel_state.dart`: 策略决策状态
- `flowpaste_panel_controller.dart`: UI 状态控制器
- `flowpaste_panel_facade.dart`: 视图绑定 facade
- `flowpaste_policy_coordinator.dart`: 偏好持久化与策略同步
- `flowpaste_panel_widget.dart`: 纯面板组件
- `flowpaste_panel_host.dart`: 生命周期接线宿主组件
