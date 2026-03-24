# flutter/flowecho_app/test

## 职责

覆盖 Flutter 模块的 contract、状态逻辑、facade、widget 与宿主接线测试。

## 关键测试文件

- `flowecho_contract_test.dart`
- `flowpaste_panel_state_test.dart`
- `flowpaste_panel_controller_test.dart`
- `flowpaste_panel_facade_test.dart`
- `flowpaste_panel_widget_test.dart`
- `flowpaste_policy_coordinator_test.dart`
- `flowpaste_panel_host_test.dart`

## 运行方式

```bash
cd flutter/flowecho_app
FLUTTER_SUPPRESS_ANALYTICS=true /opt/homebrew/bin/flutter --suppress-analytics test
```
