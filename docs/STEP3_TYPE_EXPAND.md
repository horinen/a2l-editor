# 步骤 3：类型展开

> 状态：已讨论确认
> 模块：elf.rs
> 定位：把每个 Variable 的 TypeInfo 递归展开为扁平的 A2L 条目列表

---

## 一、在管线中的位置

```
步骤 2 输出: Vec<Variable>（每个携带完整 TypeInfo）
  │
  ▼
步骤 3: 类型展开（本步骤）
  │  输入: Vec<Variable> + type_cache
  │  输出: A2lEntryStore
  ▼
步骤 4: A2L 文本生成
```

---

## 二、前置条件

| 保证 | 来源 |
|------|------|
| 每个 Variable 都有完整的 TypeInfo（非 Option） | 步骤 2 |
| TypeInfo 的所有成员 bit_offset 已是绝对 LSB | 步骤 1c |
| TypeInfo 的所有成员 type_name/type_size 已填实 | 步骤 1b |
| type_cache 中任意 type_offset 都能查到 TypeInfo | 步骤 1a |
| Typedef/const/volatile 中间节点已被拉平 | 步骤 1b |

---

## 三、核心设计原则

1. **只生成叶子节点的 entry** — Primitive/Enum/Pointer 和 bitfield 才 emit，struct/array 只做递归分发
2. **不做任何数学转换** — bit_offset 直接从 StructMember 取（步骤 1c 已规范化）
3. **ExpandContext 封装状态** — 替代当前 10 个参数的函数签名

---

## 四、ExpandContext

封装展开过程中的共享状态：

```rust
ExpandContext<'a> {
    type_cache: &'a HashMap<u64, TypeInfo>,    // 不可变，查找成员类型
    store: &'a mut A2lEntryStore,              // 可变，收集生成的 entry
    visited: HashSet<u64>,                     // 可变，循环引用检测
    root_symbol: &'a str,                      // 不可变，当前顶层变量名
    root_addr: u64,                            // 不可变，当前顶层变量地址
}
```

`type_cache` 仍然需要传入，因为 `expand_member` 用 `member.type_offset` 查 type_cache 获取完整 TypeInfo 才能递归展开。StructMember 上只有 type_offset/type_name/type_size，没有嵌入完整 TypeInfo。

---

## 五、五个展开函数

### 5.1 入口：expand_all_entries

```
输入: Vec<Variable> + type_cache
输出: A2lEntryStore

过程:
  对每个 Variable:
    创建 ExpandContext（visited=空, root_symbol=var.name, root_addr=var.address）
    expand_entry(var.name, var.address, var.type_info, depth=0, ctx)
```

### 5.2 expand_entry（调度器）

```
输入: name, addr, TypeInfo, depth, ctx

做什么:
  1. depth > 50 → 丢弃，return
  2. type_info.offset > 0 且在 visited 中 → 丢弃，return
  3. visited.insert(type_info.offset)
  4. 按 kind 分发:
     Struct | Union → expand_composite(name, addr, type_info, depth, ctx)
     Array          → expand_array(name, addr, type_info, depth, ctx)
     Primitive | Enum | Pointer | Typedef → 生成叶子 entry，添加到 ctx.store
  5. visited.remove(type_info.offset)

叶子 entry:
  A2lEntry {
      full_name: name,
      address: addr,
      size: type_info.size,
      a2l_type: infer_a2l_type_from_encoding(type_info.size, type_info.encoding),
      type_name: type_info.name,
      bit_offset: None,
      bit_size: None,
      symbol_link_name: None,
      symbol_link_offset: None,
  }
```

### 5.3 expand_composite（结构体/联合体）

```
输入: prefix, base_addr, TypeInfo(Struct|Union), depth, ctx

做什么:
  1. compute_bitfield_groups(members) → HashMap<usize, BitfieldGroup>
  2. 遍历每个 member:
     全名: member.name == "_" ? prefix : format!("{}.{}", prefix, member.name)
     bitfield   → expand_bitfield(full_name, base_addr, member, groups, ctx)
     非bitfield → expand_member(full_name, base_addr, member, depth, ctx)
```

**Struct 和 Union 处理方式完全相同。** Union 成员的 offset 恰好是 0 或子 struct 内偏移，分组和展开逻辑都能正确工作。

### 5.4 expand_bitfield（位域）

```
输入: name, base_addr, member, groups, ctx

做什么:
  1. 查 groups 获取 BitfieldGroup { container_offset, container_size }
  2. container_addr = base_addr + container_offset
  3. bit_offset = member.bit_offset    // 直接取，步骤 1c 已规范化为绝对 LSB
  4. bit_size = member.bit_size        // 直接取
  5. symbol_link_offset = container_addr - ctx.root_addr
  6. 构建 A2lEntry:
     full_name: name,
     address: container_addr,           // 容器起始地址
     size: container_size,              // 容器大小
     a2l_type: infer_a2l_type_from_encoding(container_size, ...),
     bit_offset: member.bit_offset,     // 绝对 LSB，直接取
     bit_size: member.bit_size,
     symbol_link_name: ctx.root_symbol,
     symbol_link_offset: container_addr - ctx.root_addr,
  7. 添加到 ctx.store

vs 当前的核心区别:
  - 不再做 byte_in_container * 8 + storage_bits - raw_bo - raw_bs
  - bit_offset 从 member 直接拿，步骤 1c 已经是绝对 LSB
```

### 5.5 expand_member（非位域成员）

```
输入: name, base_addr, member, depth, ctx

做什么:
  1. member_addr = base_addr + member.offset
  2. 用 member.type_offset 从 ctx.type_cache 查 TypeInfo
  3. 查到 → expand_entry(name, member_addr, resolved_type, depth+1, ctx)
  4. 查不到 → 忽略（步骤 1 的保证下理论上不应发生，可记日志）
```

### 5.6 expand_array（数组）

```
输入: prefix, base_addr, TypeInfo(Array), depth, ctx

做什么:
  1. flatten_array_type(type_info) → (dims, elem_type, elem_size)
  2. 总元素数 > MAX_ARRAY_EXPAND → 不展开
  3. 递归处理每维:
     对每个 index i:
       elem_name = format!("{}._{}_", prefix, i)
       elem_addr = base_addr + i * stride
       expand_entry(elem_name, elem_addr, elem_type, depth, ctx)
```

**flatten_array_type 保持当前逻辑不变。** 步骤 1b 的拓扑排序只处理 kind=Typedef 的节点，不修改 Array 节点的 pointer_target 链，flatten 顺着 pointer_target 链展开内层数组的行为不受影响。

---

## 六、BitfieldGroup

替代当前的 `HashMap<usize, (usize, usize)>`，显式命名容器分组信息：

```rust
struct BitfieldGroup {
    container_offset: usize,  // 容器在结构体中的字节偏移
    container_size: usize,    // 容器字节大小
}
```

### compute_bitfield_groups 算法（保持当前逻辑）

```
输入: Vec<StructMember>
输出: HashMap<usize, BitfieldGroup>  (key = member.offset)

过程:
  扫描成员列表，将连续的位域成员分为一组
  对每组:
    container_offset = min(各成员的 offset)
    container_size = max(各成员的 offset + type_size) - container_offset
  对组内每个成员:
    groups[member.offset] = BitfieldGroup { container_offset, container_size }
```

分组依据是**连续位域共享容器**。DWARF 中连续的位域成员被编译器放在同一个存储单元内，遇到非位域成员则断开。

---

## 七、只生成叶子节点的理由

| 方案 | 复杂度 | 问题 |
|------|--------|------|
| 每个层级都生成 entry（当前行为） | 高 | 需要为 struct/array 编造 a2l_type，如 620B struct 推断为 "UBYTE"（错误但未暴露） |
| 只生成叶子节点（重构后） | 低 | 叶子（Primitive/Enum/Pointer/bitfield）才 emit，中间节点只递归分发 |

bitfield 的 entry 需要容器级别的地址和大小，这个信息从 `BitfieldGroup` 获取，不需要 struct 自身的 entry。

---

## 八、删除的代码

| 函数 | 原因 |
|------|------|
| `flat_to_multi_index` | 有越界 bug（循环中 dims[i+1..] 在边界时依赖空切片 product()=1 的巧合）；重构后统一用递归路径，不再需要 |

### flat_to_multi_index 的 bug 详解

```rust
fn flat_to_multi_index(flat_idx: usize, dims: &[usize]) -> Vec<usize> {
    let mut result = Vec::with_capacity(dims.len());
    let mut remaining = flat_idx;
    for i in (1..dims.len()).rev() {
        let stride: usize = dims[i + 1..].iter().product();  // i = dims.len()-1 时越界风险
        result.push(remaining / stride);
        remaining %= stride;
    }
    result.push(remaining);  // 一维数组时循环不执行，只 push 了最后一个
}
```

当前未触发是因为大多数数组有 elem_type，走的是 `expand_multi_dim_array` 递归路径。

---

## 九、展开示例

以 `Cdd_L9388_SFR_RX`（620B struct）为例：

```
expand_entry("Cdd_L9388_SFR_RX", 0x70027000, struct_type, depth=0)
  └─ expand_composite(...)
      ├─ member WDG2TOUTTMG offset=256 非位域
      │   └─ expand_member → expand_entry("...WDG2TOUTTMG", 0x70027100, uint32, depth=1)
      │       └─ Primitive → emit: A2lEntry("...WDG2TOUTTMG", 0x70027100, 4B, ULONG)
      │
      ├─ member OPEN_LOAD offset=1 bit_offset=11 bit_size=1
      │   └─ expand_bitfield(...)
      │       └─ emit: A2lEntry("...OPEN_LOAD", container_addr, 4B, ULONG,
      │                         bit_offset=11, bit_size=1,
      │                         symbol_link="Cdd_L9388_SFR_RX", offset=容器偏移)
      │
      └─ member EXCEPTIONS_CH0_5 offset=X type=array[6] of union
          └─ expand_array(...)
              └─ for i=0..5:
                  expand_entry("...EXCEPTIONS_CH0_5._i_", addr+i*stride, union, depth+1)
                    └─ expand_composite → 每个联合体成员展开...
```

注意 struct 自身（Cdd_L9388_SFR_RX，620B）不生成 entry。

---

## 十、输出产物

### A2lEntryStore

```rust
A2lEntryStore {
    entries: Vec<A2lEntry>,
    name_index: HashMap<String, usize>,  // full_name → entries 下标
}
```

### A2lEntry 的两种形态

**普通变量（Primitive/Enum/Pointer）：**
```rust
A2lEntry {
    full_name: "Cdd_L9388_SFR_RX.WDG2TOUTTMG",
    address: 0x70027100,
    size: 4,
    a2l_type: "ULONG",
    type_name: "uint32",
    bit_offset: None,
    bit_size: None,
    symbol_link_name: None,
    symbol_link_offset: None,
}
```

**位域变量：**
```rust
A2lEntry {
    full_name: "Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD",
    address: 0x70027AAC,           // 容器起始地址
    size: 4,                       // 容器大小
    a2l_type: "ULONG",
    type_name: "uint32",
    bit_offset: Some(11),          // 绝对 LSB，直接来自步骤 1c
    bit_size: Some(1),
    symbol_link_name: Some("Cdd_L9388_SFR_RX"),  // 根符号名
    symbol_link_offset: Some(0x2AC),              // 容器在根符号中的偏移
}
```

### A2lEntry 不变量

| 属性 | 普通变量 | 位域变量 |
|------|---------|---------|
| `bit_offset` | None | Some(绝对 LSB 偏移) |
| `bit_size` | None | Some(位宽度) |
| `symbol_link_name` | None | Some(根符号名) |
| `symbol_link_offset` | None | Some(容器在根符号中的字节偏移) |
| `address` | 变量自身地址 | 容器起始地址 |
| `size` | 变量字节大小 | 容器字节大小 |

---

## 十一、讨论确认记录

- [x] ExpandContext 封装共享状态，替代 10 个参数
- [x] bit_offset 直接从 StructMember 取，不做 DWARF 数学转换（步骤 1c 已完成）
- [x] 只生成叶子节点 entry（Primitive/Enum/Pointer/bitfield）
- [x] struct/array 自身不生成 entry（避免编造错误的 a2l_type）
- [x] 删除 `flat_to_multi_index`（有越界 bug，统一用递归路径）
- [x] `flatten_array_type` 保持当前逻辑（1b 不修改 Array 的 pointer_target 链）
- [x] type_cache 仍需传入步骤 3（expand_member 按成员的 type_offset 查找完整 TypeInfo）
- [x] Union 与 Struct 处理方式相同（union 成员 offset 恰好为 0 或子 struct 内偏移）
- [x] 匿名成员（name="_"）保持前缀不变
- [x] `BitfieldGroup` 替代 `HashMap<usize, (usize, usize)>`，显式命名
