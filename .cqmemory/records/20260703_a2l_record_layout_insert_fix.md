# A2L 新平台添加变量 Record Layout 与插入位置问题

## 查询
> 新平台 ELF 添加变量时破坏原有变量结构，并报 Variable g_test_cal_valve_rz_dutyFb: Record layout _UWord_Value not found。样例：temp/dc01_opsw.elf, temp/Hcu_test.A2L。

## 摘要
目标 A2L 已有 RECORD_LAYOUT 名为 UWord/__UWORD_Z 等，旧代码导出 CHARACTERISTIC 时硬编码 __UWord_Value，且变量插入可能落在 RECORD_LAYOUT 定义区之后。已改为复用目标 A2L 现有布局名，并把新变量插入到定义区之前。

## 详情
- temp/Hcu_test.A2L 中 UWORD 标量 CHARACTERISTIC 使用 `VALUE ... UWord ...`，并在文件尾定义 `/begin RECORD_LAYOUT UWord`。
- 原代码 `A2lGenerator::get_record_layout` 返回 `__UWord_Value`，目标文件没有该 RECORD_LAYOUT，因此 CANape/ASAP2 工具报 layout not found。
- 原 append/add 插入逻辑优先 `/begin GROUP`，否则靠最后变量或 `/end MODULE`，在该样例没有 GROUP 时会把新变量放到 RECORD_LAYOUT 区之后，容易打乱工具期望的 A2L 块顺序。
- 修复后 `append_to_file` 和 `apply_changes(add)` 会解析目标 A2L 的 RECORD_LAYOUT 名，并优先选择该文件已有的布局名；变量插入位置改为最后一个变量块之后、COMPU/RECORD_LAYOUT/GROUP 等定义区之前。
- 验证：`cargo test` 28 passed；临时导出 `/tmp/Hcu_test_export_check.A2L` 中 `g_test_cal_valve_rz_dutyFb` 生成为 `VALUE 0x20000026 UWord 0 NO_COMPU_METHOD 0 65535`，且早于 RECORD_LAYOUT 区。

## 相关文件
| 文件 | 行号 | 说明 |
|------|------|------|
| src/lib/a2l.rs | 312 | CHARACTERISTIC 生成支持传入 record layout |
| src/lib/a2l.rs | 414 | 解析/选择目标 A2L 已有 RECORD_LAYOUT，并计算变量插入位置 |
| src/lib/a2l.rs | 624 | append_to_file 使用目标文件布局名和新插入位置 |
| src/lib/a2l.rs | 1067 | save_a2l_changes 的 add 路径使用同样逻辑 |

## 元数据
- created: 2026-07-03T00:00:00+08:00
- project: a2l-editor
- tags: [a2l, record-layout, export, characteristic]
