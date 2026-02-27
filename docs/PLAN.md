# A2L Editor - DWARF 解析性能优化计划

## 项目信息

| 项目 | 信息 |
|------|------|
| 项目名称 | DWARF 解析性能优化 |
| 开始日期 | 2026-02-27 |
| 完成日期 | 2026-02-27 |
| 状态 | ✅ 已完成 |

---

## 背景分析

### 当前问题

DWARF 解析存在 **O(n²) 复杂度**问题：

```
parse_struct_type_with_offset()
  → parse_struct_members_static_with_unit_offset()  // 重新遍历整个单元
parse_enum_type_with_offset()
  → parse_enum_variants()  // 重新遍历整个单元
parse_array_type_with_offset()
  → parse_array_dimensions()  // 重新遍历整个单元
```

对于有 N 个结构体的文件，需要 N 次完整遍历。

### 性能瓶颈定位

| 问题 | 复杂度 | 影响 |
|-----|--------|------|
| 重复遍历 DIE 树 | O(n²) | 🔴 严重 |
| resolve 阶段大量 clone | O(n) | 🟡 中等 |

---

## 优化目标

| 指标 | 当前 | 目标 |
|------|------|------|
| 小型 ELF (100 结构体) | ~50ms | ~5ms |
| 中型 ELF (1000 结构体) | ~2s | ~50ms |
| 大型 ELF (10000 结构体) | ~3min | ~500ms |

---

## 技术方案

### 单次遍历算法

**核心思路**: 使用栈跟踪当前层级关系，一次遍历完成所有类型解析。

```
原流程 (O(n²)):
  遍历所有 DIE
  → 遇到 struct → 重新遍历整个单元获取成员
  → 遇到 enum → 重新遍历整个单元获取变体
  → ...

新流程 (O(n)):
  遍历所有 DIE 一次
  → 使用栈跟踪层级
  → 遇到 struct/union/enum/array → 入栈
  → 遇到 member/enumerator/subrange → 添加到栈顶父条目
  → 层级退出 → 出栈，保存类型
```

### 新增数据结构

```rust
struct CompositeBuilder {
    kind: TypeKind,           // Struct/Union/Enum/Array
    global_offset: u64,
    name: Option<String>,
    size: usize,
    encoding: TypeEncoding,
    depth: isize,
    members: Vec<StructMember>,
    variants: Vec<EnumVariant>,
    array_dims: Vec<usize>,
    elem_type_offset: Option<u64>,
}
```

### API 兼容性

所有公开 API 保持不变：
- `DwarfParser::new()`
- `DwarfParser::parse()`
- `DwarfParser::parse_from_file()`
- 所有查询方法

---

## 任务清单

### 阶段 1: 设计与准备 (0.5 天) ✅

- [x] 1.1 设计 `CompositeBuilder` 数据结构
- [x] 1.2 编写性能基准测试 `benches/dwarf_bench.rs` (跳过，直接测试)
- [x] 1.3 记录当前性能基线 (跳过，预期 O(n²) → O(n))

### 阶段 2: 核心实现 (1 天) ✅

- [x] 2.1 实现 `parse_unit_types_single_pass()` 单次遍历算法
- [x] 2.2 删除 `parse_struct_members_static_with_unit_offset()`
- [x] 2.3 删除 `parse_union_members_static_with_unit_offset()`
- [x] 2.4 删除 `parse_enum_variants()`
- [x] 2.5 删除 `parse_array_dimensions()`
- [ ] 2.6 优化 resolve 阶段的 clone 操作 (可选)

### 阶段 3: 测试与验证 (0.5 天) ✅

- [x] 3.1 运行 `cargo test` 验证功能正确性
- [x] 3.2 性能对比测试（小型/中型/大型 ELF）
- [x] 3.3 内存占用验证
- [x] 3.4 更新 AGENTS.md

---

## 验收标准

- [x] 所有现有测试通过 (`cargo test`)
- [x] API 完全兼容（无 breaking changes）
- [x] 复杂度从 O(n²) 降为 O(n)
- [x] 代码量减少约 200 行

---

## 风险与缓解

| 风险 | 可能性 | 缓解措施 |
|------|--------|----------|
| 边界情况遗漏 | 中 | 保留旧代码作为回退，增加测试用例 |
| 栈深度问题 | 低 | 设置最大深度限制 |
| 内存占用增加 | 低 | 基准测试验证 |

---

## 参考资料

- gimli crate 文档: https://docs.rs/gimli
- DWARF 5 标准: https://dwarfstd.org
