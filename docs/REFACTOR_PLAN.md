# ELF 解析管线重构方案

> 状态：方案讨论阶段，待逐一确认后执行
> 背景：ELF 解析模块反复出现 bug（见 ISSUES.md），根因不是单点逻辑错误，而是架构层面的结构性问题。Bug 已全部修复，本方案是防御性重构。

---

## 一、已有 bug 复盘

共 5 个 bug，全部已修复。以下是模式总结：

| Bug | 文件 | 根因归类 |
|-----|------|---------|
| 匿名 struct 成员跳过 | dwarf.rs | 类型解析遗漏（只遍历 struct_map） |
| 位域容器地址/大小错误 | elf.rs | expand_recursive 中位域用了 member.offset 而非容器起始 |
| ULEB128 解码错误 | dwarf.rs | DWARF 编码细节泄漏到业务逻辑 |
| SYMBOL_LINK 指向点分路径 | elf.rs + a2l.rs | 位域的根符号信息跨多层传递断裂 |
| DWARF2 位域偏移转换错误 | elf.rs + types.rs | bit_offset 在 3 个文件间被反复转换，每层都"以为"上一层做了 |

**结论：** 问题集中在 `dwarf.rs`（类型解析不完整）和 `elf.rs`（expand_recursive 职责过多），以及两者之间的数据边界不清晰。

---

## 二、架构问题诊断

### 问题 A：expand_recursive 职责爆炸

`elf.rs:338-489`，约 150 行，同时处理 6 种关注点：

- 递归深度控制 / 循环检测（depth, visited）
- Struct/Union 成员遍历与名称拼接
- Array 多维展开（flatten + multi_dim）
- Bitfield 位偏移转换（DWARF2 → 绝对 LSB）
- Symbol Link 根符号追踪（root_symbol, root_addr）
- 容器分组计算（compute_bitfield_groups）

**后果：** 每修一个分支，可能影响其他分支。10 个参数的函数签名，难以理解和测试。

### 问题 B：位域数据无规范化层

位域偏移经历了 3 次变换，横跨 3 个文件：

```
dwarf.rs                    elf.rs                      a2l.rs
parse_member()              expand_recursive()          generate_*_block()
     │                           │                          │
DW_AT_bit_offset                │                     BIT_MASK 计算
(raw, DWARF2 MSB)               │                     用 effective_bit_offset
     │                          │                          │
     ▼                          ▼                          ▼
StructMember.bit_offset    actual_bit_offset =         get_effective_bit_offset()
(原样存储)                 byte_in_container*8          (又做了一次转换)
                           + storage_bits
                           - raw_bo - raw_bs
                                │
                                ▼
                           A2lEntry.bit_offset
                           (应该是绝对 LSB)
```

**后果：** 没有定义"位域信息"应该在哪个层级完成规范化，导致层与层之间互相猜测。

### 问题 C：类型解析用迭代式"碰运气"

dwarf.rs 三个 resolve 函数都是多次迭代直到不变：

- `resolve_type_refs` — 最多 100 次迭代
- `resolve_array_element_types` — 最多 10 次迭代
- `resolve_all_member_types` — 单次遍历

**后果：** 对长链引用（const→typedef→typedef→actual_type）可能解析不到。匿名类型可能被跳过。

### 问题 D：零测试覆盖

全项目 0 个 `#[test]`。每次改动靠人肉验证，无法防止回归。

---

## 三、重构方案：分层拆分

### 核心原则

1. **每种类型的展开逻辑独立可测**
2. **DWARF 原始数据在边界层（dwarf.rs）一次性规范化，下游只消费**
3. **A2L 生成层变成纯格式化器，不做任何计算**

### 层级关系

```
第 0 层：数据模型定义（types.rs）—— 明确字段语义
    │
    ├─→ 第 1 层：DwarfParser 规范化（dwarf.rs）—— 输出"已解析完毕"的类型系统
    │
    ├─→ 第 2 层：类型展开器（elf.rs）—— 拆分 expand_recursive 为独立函数
    │
    └─→ 第 3 层：A2L 生成（a2l.rs）—— 简化为纯格式化器
```

每层可独立测试，不需要真实 ELF 文件（用 fixture 构造 TypeInfo/StructMember 即可）。

---

## 四、各层详细方案

> 以下每一节需要在新会话中逐一讨论确认。

### 第 0 层：数据模型（types.rs）

**目标：** 明确每个字段的语义，消除"这个字段存的是什么"的歧义。

#### StructMember 的字段语义

| 字段 | 当前含义 | 重构后含义 | 谁填充 | 谁消费 |
|------|---------|-----------|--------|--------|
| `offset` | DWARF data_member_location | **绝对字节偏移**（已 ULEB128 解码） | dwarf.rs | elf.rs |
| `bit_offset` | DWARF2 DW_AT_bit_offset（从存储单元 MSB 算） | **绝对 LSB 位偏移**（从容器起始字节算，bit 0 = byte 0 的 bit 0） | dwarf.rs | elf.rs |
| `bit_size` | DW_AT_bit_size | 不变 | dwarf.rs | elf.rs |
| `type_offset` | DWARF 类型引用偏移 | **保留**，作为 type_cache 的已验证查找 key | dwarf.rs | elf.rs |
| `type_name` | 可能是 "unknown" | **已解析的真实类型名** | dwarf.rs | elf.rs + a2l.rs |
| `type_size` | 可能是 0 | **已解析的真实类型大小** | dwarf.rs | elf.rs |

**关键变化：** `bit_offset` 从"DWARF 原始值"变为"绝对 LSB 偏移"。下游不再需要知道 DWARF2/DWARF4 的区别。

#### A2lEntry 的字段语义

| 字段 | 语义 | 谁填充 |
|------|------|--------|
| `bit_offset` | 绝对 LSB 位偏移，从容器起始字节算 | elf.rs expand_bitfield |
| `bit_size` | 位域位数 | elf.rs expand_bitfield |
| `symbol_link_name` | ELF 根符号名（如 `Cdd_L9388_SFR_RX`），仅 bitfield 设置 | elf.rs |
| `symbol_link_offset` | 容器在根符号中的字节偏移，仅 bitfield 设置 | elf.rs |

**关键变化：** 删除 `get_effective_bit_offset()` 方法。bit_offset 已是最终形态，a2l.rs 直接用它算 BIT_MASK。

#### 新增类型

- `BitfieldGroup { container_offset: usize, container_size: usize }` — 替代当前的 `HashMap<usize, (usize, usize)>`，显式命名容器分组信息

---

### 第 1 层：DwarfParser 规范化（dwarf.rs）

**目标：** DwarfParser 的输出是"已解析完毕"的类型系统，下游不需要理解 DWARF 细节。

#### 当前流程

```
parse_dwarf_sections()
  ├─ parse_unit_types()           ← 第 1 遍：遍历 DIE，填充 type_cache
  ├─ resolve_all_member_types()   ← 第 2 遍：成员 type_offset → type_name/type_size
  ├─ resolve_type_refs()          ← 第 3 遍：typedef/const/volatile → 实际类型（迭代 100 次）
  └─ resolve_array_element_types()← 第 4 遍：数组元素类型（迭代 10 次）
```

#### 重构后流程

```
parse_dwarf_sections()
  ├─ parse_unit_types()           ← 不变
  ├─ resolve_type_graph()         ← 替代原来的 3 个 resolve 函数
  └─ normalize_bitfield_offsets() ← 新增
```

#### resolve_type_graph — 拓扑排序替代迭代

**输入：** type_refs + type_cache + array_elem_offsets

**做什么：**
1. 收集所有需要解析的 offset（typedef/const/volatile/array）
2. 构建依赖 DAG：如果 A 指向 B，则 A 依赖 B
3. 拓扑排序
4. 按序解析每个节点：从 type_cache 取目标类型，复制 size/encoding/kind/members 到当前节点
5. 检测循环引用并报错

**vs 当前的区别：** 迭代式最多 100 次但可能不收敛（循环引用）。拓扑排序保证一次成功。

#### normalize_bitfield_offsets — 位域偏移一次性规范化

**做什么：**
- 遍历 type_cache 中所有 Struct/Union 的成员
- 对每个 bitfield 成员：
  - DWARF2 格式（有 DW_AT_bit_offset）：
    ```
    absolute_lsb = member.offset * 8 + (member.type_size * 8 - member.bit_offset - member.bit_size)
    ```
  - DWARF4 格式（有 DW_AT_data_bit_offset）：
    ```
    absolute_lsb = DW_AT_data_bit_offset  （已是绝对偏移，直接用）
    ```
- 原地替换 member.bit_offset = absolute_lsb

**示例（OPEN_LOAD）：**
```
输入：offset=1, bit_offset=4(DWARF2 MSB), bit_size=1, type_size=1(uint8)
计算：1 * 8 + (8 - 4 - 1) = 8 + 3 = 11
输出：bit_offset = 11
```

#### 第 1 层输出物

```
DwarfParser.parse() 最终输出：
├─ type_cache — 所有成员 bit_offset 已是绝对 LSB，所有 type_name/type_size 已解析
├─ variable_types — 变量名 → type_cache key
├─ global_variables — 变量名 + 地址
└─ struct_map — 命名结构体副本
```

**关键约定：** 从 DwarfParser 出来的数据，不包含任何 DWARF 编码细节。

---

### 第 2 层：类型展开器（elf.rs）

**目标：** 把 expand_recursive 拆成职责单一的管线。

#### ExpandContext 结构

封装展开过程中的共享状态，替代当前 10 个参数的函数签名：

```
ExpandContext {
    type_cache: &HashMap<u64, TypeInfo>     // 不可变，查找成员类型
    store: &mut A2lEntryStore               // 可变，收集生成的 entry
    visited: HashSet<u64>                   // 可变，循环引用检测
    root_symbol: &str                       // 不可变，当前顶层变量名
    root_addr: u64                          // 不可变，当前顶层变量地址
}
```

#### 展开管线

```
入口：expand_variable(var, type_cache) → A2lEntryStore
  │
  ├─ 查 type_cache 获取变量的 TypeInfo
  │
  └─ expand_entry(name, addr, type, depth=0)
      │
      ├─ 深度检查 + 循环检查
      │
      ├─ 总是向 store 添加自身 entry
      │   （name, addr, type.size → A2lEntry）
      │
      └─ 按 type.kind 分发：
          │
          ├─ Struct | Union → expand_composite(prefix, base, type, depth)
          │   ├─ compute_bitfield_groups(members)
          │   └─ 遍历成员：
          │       ├─ bitfield → expand_bitfield(name, base, member, groups)
          │       └─ 非bitfield → expand_member(name, base, member, depth)
          │
          ├─ Array → expand_array(prefix, base, type, depth)
          │   ├─ flatten_array_type → (dims, elem_type, elem_size)
          │   ├─ 检查总元素数 <= MAX_ARRAY_EXPAND
          │   └─ 递归展开每维：prefix._i_ + 偏移
          │
          └─ Primitive | Enum | Pointer | Typedef → 无子节点，结束
```

#### expand_entry（调度器）

**输入：** name, addr, TypeInfo, depth

**做什么：**
1. depth > 50 → 丢弃
2. offset 在 visited 中 → 丢弃
3. 向 store 添加 A2lEntry（name, addr, size, a2l_type, type_name）
4. 按 kind 分发到 expand_composite / expand_array
5. 退出时从 visited 移除

**生成物：** 自身的 1 个 A2lEntry + 子节点展开产生的若干 A2lEntry

**待确认问题：** 步骤 3 会为 struct/array 自身也生成 entry（如 `Cdd_L9388_SFR_RX` 自身有一个 620B 的 entry，同时它的成员也各有 entry）。是否需要保留？

#### expand_composite（结构体/联合体）

**输入：** prefix, base_addr, TypeInfo(Struct|Union), depth

**做什么：**
1. 调用 compute_bitfield_groups → `HashMap<usize, BitfieldGroup>`
2. 遍历每个 member：
   - 计算全名：匿名成员用父前缀，否则 `prefix.name`
   - bitfield → expand_bitfield
   - 非bitfield → expand_member（查 type_cache 递归展开）

**生成物：** 每个成员对应的若干 A2lEntry

#### expand_bitfield（位域）

**输入：** name, base_addr, member, groups

**做什么：**
1. 查 groups 获取 BitfieldGroup { container_offset, container_size }
2. container_addr = base_addr + container_offset
3. 从 member.bit_offset 直接取值（已是绝对 LSB，第 1 层完成）
4. 从 member.bit_size 直接取值
5. symbol_link_offset = container_addr - root_addr
6. 构建 A2lEntry：
   - address = container_addr
   - size = container_size
   - a2l_type = 由 container_size 推断
   - bit_offset = member.bit_offset
   - bit_size = member.bit_size
   - symbol_link_name = root_symbol
   - symbol_link_offset = container_addr - root_addr
7. 添加到 store

**生成物：** 1 个 A2lEntry

**vs 当前的核心区别：** 不再做 DWARF2 → LSB 的数学运算，只负责容器分组 + SYMBOL_LINK。

#### expand_member（非位域成员）

**输入：** name, base_addr, member, depth

**做什么：**
1. member_addr = base_addr + member.offset
2. 用 member.type_offset 查 type_cache
3. 找到 → 调用 expand_entry(name, member_addr, resolved_type, depth+1)
4. 找不到 → 忽略

**生成物：** 由 expand_entry 递归产生的若干 A2lEntry

#### expand_array（数组）

**输入：** prefix, base_addr, TypeInfo(Array), depth

**做什么：**
1. flatten_array_type → (effective_dims, elem_type, elem_size)
2. 检查总元素数 <= MAX_ARRAY_EXPAND
3. 完全用递归处理多维数组，删除 flat_to_multi_index

**vs 当前的核心区别：** 统一为一条递归路径，删除当前的 flat_to_multi_index（有越界风险）。

#### 展开示例

```
Variable "Cdd_L9388_SFR_RX" addr=0x70027000, type=Cdd_L9388_Register_Type(620B struct)
  │
  ▼ expand_entry("Cdd_L9388_SFR_RX", 0x70027000, struct_type, depth=0)
  ├─ emit: A2lEntry("Cdd_L9388_SFR_RX", 0x70027000, 620B, ULONG)
  └─ expand_composite(...)
      ├─ member WDG2TOUTTMG offset=256 (非位域)
      │   └─ expand_entry("...WDG2TOUTTMG", 0x70027100, uint32, depth=1)
      │       └─ emit: A2lEntry("...WDG2TOUTTMG", 0x70027100, 4B, ULONG)
      │
      ├─ member OPEN_LOAD offset=1 bit_offset=11 bit_size=1 (位域)
      │   └─ expand_bitfield(...)
      │       └─ emit: A2lEntry("...OPEN_LOAD", 0x70027AAC, 4B, ULONG,
      │                         bit_offset=11, bit_size=1,
      │                         symbol_link="Cdd_L9388_SFR_RX", offset=0x2AC)
      │
      └─ member EXCEPTIONS_CH0_5 offset=X type=array[6] of union
          └─ expand_array(...)
              └─ for i=0..5: expand_entry → expand_composite → ...
```

---

### 第 3 层：A2L 生成（a2l.rs）

**目标：** 变成纯格式化器，不做任何计算。

#### generate_measurement_block 的变化

**当前：**
1. 取 entry.bit_offset
2. 调用 entry.get_effective_bit_offset(endianness) 做转换 ← 删除
3. 计算 BIT_MASK
4. 输出 ECU_ADDRESS / SYMBOL_LINK

**重构后：**
1. entry.is_bitfield() → 直接取 entry.bit_offset → 算 BIT_MASK
2. entry 有 symbol_link_name → 输出 `SYMBOL_LINK "name" offset`
3. 否则 → 输出 `SYMBOL_LINK "entry.full_name" 0`

**endianness 参数可能可以移除**（需确认 A2L 规范中 BIT_MASK 是否字节序无关）。

---

## 五、测试策略

### 分层测试

```
unit: dwarf.rs
├── ULEB128 解码（用已知字节序列验证）
├── bit_offset 规范化（构造 DWARF2 数据验证 LSB 转换）
├── type_ref 拓扑排序（构造 typedef 链验证）
└── DWARF4 data_bit_offset 直接使用

unit: elf.rs (ExpandContext)
├── 简单 struct 展开（3 个成员 → 3 个 entry）
├── 嵌套 struct 展开（struct 内含 struct）
├── 一维 / 多维 / 嵌套数组展开
├── bitfield 展开（同一容器多成员 → 共享地址）
├── bitfield + array（struct 内含 bitfield + array 混合）
├── 递归深度限制
├── 循环引用检测
└── 匿名成员展开

unit: a2l.rs
├── MEASUREMENT block 生成（含/不含 bitfield）
├── CHARACTERISTIC block 生成
├── COMPU_METHOD block 生成
└── SYMBOL_LINK 生成（bitfield vs 普通）

integration: 全管线
├── 构造 TypeInfo fixture → expand_variable → 对比 A2lEntryStore
├── 构造 A2lEntry → generate_measurement_block → 对比文本 snapshot
└── 用 ISSUES.md 中每个 bug 的复现变量作回归测试
```

### 测试方式

不需要真实 ELF 文件。构造 `TypeInfo` / `StructMember` 的 fixture 直接测试展开逻辑。

---

## 六、执行顺序

| 阶段 | 内容 | 风险 | 依赖 |
|------|------|------|------|
| Phase 0 | 写测试覆盖当前行为 | 低 | 无 |
| Phase 1 | dwarf.rs 规范化（拓扑排序 + bitfield normalize） | 中 | Phase 0 |
| Phase 2 | elf.rs 拆分（ExpandContext） | **高** | Phase 1 |
| Phase 3 | a2l.rs 简化 | 低 | Phase 2 |

Phase 2 之前，建议用 CLI 导出一份真实 ELF 的 A2lEntryStore 作为 baseline，重构后对比。

---

## 七、待确认问题

讨论时逐个确认：

- [ ] **expand_entry 是否为 struct/array 自身也生成 entry？** 当前行为是生成（如 `Cdd_L9388_SFR_RX` 自身有一个 entry）。如果不需要，只保留叶子节点，逻辑更简单。
- [ ] **endianness 与 BIT_MASK 的关系？** A2L 规范中 BIT_MASK 是否字节序无关？决定了 endianness 是只留在第 1 层还是需要透传到第 3 层。
- [ ] **DWARF4 的 DW_AT_data_bit_offset 是否需要支持？** 当前不支持。如果需要，在 normalize_bitfield_offsets 中添加。
- [ ] **测试是否需要 CI 集成？** 还是只在本地跑 `cargo test`。

---

## 八、讨论进度

在后续会话中逐模块讨论，每完成一个模块在此记录结论。

- [ ] 第 0 层：数据模型讨论
- [ ] 第 1 层：dwarf.rs 讨论与确认
- [ ] 第 2 层：elf.rs 讨论与确认
- [ ] 第 3 层：a2l.rs 讨论与确认
- [ ] 测试策略讨论与确认
