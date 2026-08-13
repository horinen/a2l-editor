# Cdd_WheelSpeed_Fault 搜索结果错误

## 查询
> 理解 A2L Editor 项目，调查 temp/test.elf 中 Cdd_WheelSpeed_Fault 搜索结果不正确。

## 摘要
旧数据包把 Cdd_WheelSpeed_Fault 的二维数组元素当成 unknown/UWORD 展开，根因是 DWARF const/typedef 等 size=0 别名链没有迭代解析到最终匿名 struct，导致数组元素类型丢失。

## 详情
- 旧包搜索结果：26 条 Cdd_WheelSpeed_Fault._x_._y_ UWORD。
- DWARF 真实类型：array[2][13]，内层元素是 2 字节匿名结构体，成员为 FltDebValue bit[0,13] 和 CurrentDebStatus bit[14,15]。
- 修复点：src/lib/dwarf.rs 在 resolve_type_graph() 后新增 resolve_alias_chains()，反复把 size=0 的类型引用解析到最终非零大小目标，再执行 resolve_array_element_types()。
- 修复后重新生成 temp/test.elf.a2ldata，搜索结果变为 52 条位域字段。

## 相关文件
| 文件 | 行号 | 说明 |
|------|------|------|
| src/lib/dwarf.rs | 307 | 调用 resolve_alias_chains |
| src/lib/dwarf.rs | 348 | 别名链解析实现 |
| src/lib/dwarf.rs | 1418 | 单元测试覆盖数组元素别名链解析 |

## 验证
```
cargo test
cargo run --bin a2l-cli -- entries temp/test.elf Cdd_WheelSpeed_Fault -n 8
```

## 元数据
- created: 2026-06-04T19:08:00+08:00
- project: a2l-editor
- tags: [dwarf, a2l, search, bitfield, array]
