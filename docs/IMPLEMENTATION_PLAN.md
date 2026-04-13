# ELF 解析管线重构 — 实施计划

> 本文档是给执行重构的 AI 会话的指令文档。
> 请先阅读以下讨论文档，理解设计决策的背景：
> - docs/STEP1_DWARF_PARSE.md — 步骤 1：DWARF 解析 + 规范化
> - docs/STEP2_VARIABLE_EXTRACT.md — 步骤 2：变量提取
> - docs/STEP3_TYPE_EXPAND.md — 步骤 3：类型展开
> - docs/STEP4_A2L_GENERATE.md — 步骤 4：A2L 文本生成
> - docs/REFACTOR_PLAN.md — 原始重构方案（背景参考）
> - docs/ISSUES.md — 历史 bug 记录（理解问题根源）

---

## 一、执行顺序

按以下顺序逐阶段实施，每个阶段完成后运行 `cargo build && cargo test` 验证。

### Phase 0: 数据模型变更（types.rs） ✅ 已完成

**文件: `src/lib/types.rs`**

1. **Variable 结构体变更：**
   - 删除 `section` 字段
   - `type_info` 从 `Option<TypeInfo>` 改为 `TypeInfo`
   - 对应修改 `Variable::new()` 签名（去掉 section 参数）和 `with_type_info()` 方法（改为直接赋值）
   - 代码注释使用中文

2. **新增 BitfieldGroup 结构体：**
   ```rust
   pub struct BitfieldGroup {
       pub container_offset: usize,
       pub container_size: usize,
   }
   ```

3. **删除转换函数：**
   - 删除 `StructMember::get_effective_bit_offset()`
   - 删除 `A2lEntry::get_effective_bit_offset()`

4. **同步修改受影响的文件：**
   - `src/bin/a2l_cli.rs` — 删除 `section_dist` 相关统计代码，修改 `Variable::new()` 调用处去掉 section 参数
   - `src/lib/elf.rs` — 修改 `Variable::new()` 调用处
   - `src/lib/a2l.rs` — 两处 `get_effective_bit_offset(endianness)` 调用替换为 `entry.bit_offset.unwrap()`
   - `src-tauri/src/commands.rs` — 检查 `EntryInfo::from` 实现，确保 `symbol_link` 字段仍正确映射

5. **验证：** `cargo build`（当前无测试，编译通过即可）

---

### Phase 1: dwarf.rs 规范化 ✅ 已完成

**文件: `src/lib/dwarf.rs`**

1. **替换 resolve 函数为 resolve_type_graph：**
   - 删除 `resolve_type_refs()`、`resolve_array_element_types()`、`resolve_all_member_types()` 三个函数
   - 新增 `resolve_type_graph()` 方法：
     - 收集所有 kind=Typedef 的节点
     - 构建 DAG（节点 → target offset）
     - 拓扑排序（检测循环引用则报错）
     - 按序解析：从 type_cache 取目标类型，复制 size/encoding/kind/members，**name 保留自身**
     - 第二轮：遍历所有 Struct/Union 的成员，用 member.type_offset 从 type_cache 查 TypeInfo，填充 member.type_name 和 member.type_size
   - 更新 `parse_dwarf_sections()` 调用新方法

2. **新增 normalize_bitfield_offsets()：**
   - 遍历 type_cache 中 kind=Struct/Union 的所有节点
   - 对每个 bitfield 成员：
     - DWARF2 格式：`absolute_lsb = member.offset * 8 + (member.type_size * 8 - member.bit_offset - member.bit_size)`
     - DWARF4 格式：直接用 `data_bit_offset`（如果支持的话）
     - 原地替换 `member.bit_offset = absolute_lsb`
   - 在 `parse_dwarf_sections()` 中 resolve_type_graph 之后调用

3. **ULEB128 解码修复（已修复，确认代码正确即可）：**
   - 确认 `get_member_location_static` 中 `read_uleb128` 的使用是否已就位
   - 如果还没改，确保 Block 和 Exprloc 分支都使用 `read_uleb128`

4. **验证：** `cargo build`

---

### Phase 2: elf.rs 拆分（高风险）

**文件: `src/lib/elf.rs`**

**前置准备：用 CLI 导出 baseline**
```bash
cargo run --bin a2l-cli -- deep <test.elf_path>
# 记录条目数、几个已知变量的地址/bit_offset 等作为 baseline
```

1. **删除不再需要的函数：**
   - `extract_variables_from_elf()` — 无 DWARF 路径删除
   - `enrich_variables_with_types()` — 合并进 extract
   - `infer_type_name()` — 无 DWARF 路径删除
   - `flat_to_multi_index()` — 有 bug，统一用递归路径

2. **修改 parse_with_depth：**
   - deep=true 时，如果 DwarfParser 没有 DWARF 信息，返回错误而非退化为符号表路径
   - 合并 extract + enrich 为一步调用

3. **重写 extract_variables_from_dwarf：**
   - 合并原 extract + enrich 的逻辑
   - type_offset 查找失败时报错（而非静默跳过）
   - 输出 Vec<Variable>，每个的 type_info 是必选的（不是 Option）

4. **引入 ExpandContext：**
   ```rust
   struct ExpandContext<'a> {
       type_cache: &'a HashMap<u64, TypeInfo>,
       store: &'a mut A2lEntryStore,
       visited: HashSet<u64>,
       root_symbol: &'a str,
       root_addr: u64,
   }
   ```

5. **拆分 expand_recursive 为五个函数：**
   - `expand_entry(name, addr, type_info, depth, ctx)` — 调度器
     - depth > 50 → return
     - visited 检测 → return
     - Struct/Union → expand_composite
     - Array → expand_array
     - **Primitive/Enum/Pointer/Typedef → 生成叶子 entry（emit）**
     - **struct/array 自身不生成 entry**
   - `expand_composite(prefix, base_addr, type_info, depth, ctx)`
     - compute_bitfield_groups（返回 HashMap<usize, BitfieldGroup>）
     - 遍历成员，匿名成员保持前缀，有名成员拼接
     - bitfield → expand_bitfield，非 bitfield → expand_member
   - `expand_bitfield(name, base_addr, member, groups, ctx)`
     - 从 groups 获取 BitfieldGroup
     - **bit_offset 直接取 member.bit_offset（步骤 1c 已规范化，不做 DWARF 数学运算）**
     - 设置 symbol_link_name = ctx.root_symbol, symbol_link_offset = container_addr - ctx.root_addr
   - `expand_member(name, base_addr, member, depth, ctx)`
     - member_addr = base_addr + member.offset
     - 用 member.type_offset 查 ctx.type_cache → expand_entry
   - `expand_array(prefix, base_addr, type_info, depth, ctx)`
     - flatten_array_type 保持不变
     - 递归处理每维（保留 expand_multi_dim_array 的递归模式）

6. **compute_bitfield_groups 返回类型变更：**
   - 从 `HashMap<usize, (usize, usize)>` 改为 `HashMap<usize, BitfieldGroup>`
   - 算法不变

7. **修改 expand_variable：**
   - 删除 `type_info = None` 的 else 分支（重构后不可能为 None）
   - 创建 ExpandContext，调用 expand_entry

8. **验证：**
   - `cargo build && cargo test`
   - 用同一个测试 ELF 重新导出，对比条目数和关键变量的地址/bit_offset
   - **预期差异：** 条目数可能减少（不再为 struct/array 自身生成 entry），但所有叶子 entry 的地址和 bit_offset 应完全一致

---

### Phase 3: a2l.rs 简化

**文件: `src/lib/a2l.rs`**

1. **修改 calculate_bit_mask 签名：**
   ```rust
   fn calculate_bit_mask(bit_offset: usize, bit_size: usize, container_size_bits: usize, endianness: Endianness) -> u64 {
       let shift = match endianness {
           Endianness::Little => bit_offset,
           Endianness::Big => container_size_bits - bit_offset - bit_size,
       };
       ((1u64 << bit_size) - 1) << shift
   }
   ```

2. **修改 generate_measurement_block_with_compu：**
   - 删除 `symbol_link` 参数
   - BIT_MASK 计算：直接用 `entry.bit_offset.unwrap()` 传给 calculate_bit_mask
   - SYMBOL_LINK：从 entry.symbol_link_name/offset 取值，不再依赖外部参数

3. **修改 generate_characteristic_block_with_compu：**
   - 同上

4. **同步修改所有调用点：**
   - `append_to_file` — 更新调用签名
   - `apply_changes` — 更新调用签名
   - `generate` — 更新调用签名

5. **验证：** `cargo build && cargo test`

---

### Phase 4: 最终验证

```bash
# 编译
cargo build

# 测试
cargo test

# CLI 集成验证
cargo run --bin a2l-cli -- deep <test.elf_path>

# 前端类型检查（在 src-ui/ 目录下）
cd src-ui && npm run check

# Tauri 完整构建（可选，验证 IPC 层不受影响）
cd /home/hori/project/a2l-editor && npm run tauri build
```

---

## 二、不变量检查清单

每个 Phase 完成后，确认以下不变量：

### 步骤 1 输出不变量
- [ ] type_cache 中所有节点的 name ≠ "unknown"、size > 0
- [ ] Struct/Union 成员的 type_name/type_size 已填实
- [ ] bitfield 成员的 bit_offset 是绝对 LSB（不是 DWARF 原始值）
- [ ] 匿名成员名 = "_"，不被跳过

### 步骤 2 输出不变量
- [ ] 每个 Variable 的 type_info 是必选的（不是 Option）
- [ ] Variable.size = Variable.type_info.size
- [ ] 变量名唯一

### 步骤 3 输出不变量
- [ ] 只有叶子节点（Primitive/Enum/Pointer/bitfield）生成 entry
- [ ] struct/array 自身不生成 entry
- [ ] 位域 entry 的 bit_offset 直接来自 StructMember（绝对 LSB）
- [ ] 位域 entry 的 symbol_link_name = 根变量名
- [ ] 非位域 entry 无 bit_offset/bit_size/symbol_link

### 步骤 4 输出不变量
- [ ] BIT_MASK 计算正确处理 endianness
- [ ] 位域 SYMBOL_LINK 指向根符号 + 容器偏移
- [ ] 非位域 SYMBOL_LINK 指向 full_name + 0

---

## 三、已知风险

| 风险 | 缓解措施 |
|------|---------|
| Phase 2 改动量大，可能引入新 bug | 先导出 baseline，重构后逐条对比 |
| resolve_type_graph 的拓扑排序可能有边界情况 | 对同一 ELF 对比重构前后的 type_cache 输出 |
| 删除 struct/array 自身 entry 可能影响前端功能 | 前端只消费 EntryInfo，不依赖"容器级"entry |

---

## 四、不在重构范围内的内容

以下功能/文件不需要修改：
- DataPackage（SQLite 持久化）
- commands.rs（IPC 层）
- 前端代码（src-ui/）
- A2L 编辑功能（remove_variables / modify_variable / apply_changes）
- A2L 解析功能（A2lParser）
- COMPU_METHOD 生成
