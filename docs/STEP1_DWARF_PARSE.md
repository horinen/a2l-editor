# 步骤 1：DWARF 解析 + 规范化

> 状态：已讨论确认
> 模块：dwarf.rs
> 定位：整个管线中唯一接触 DWARF 编码细节的步骤

---

## 一、在管线中的位置

```
ELF 文件
  │
  ▼
步骤 1: DWARF 解析 + 规范化（本步骤）  ← 唯一接触 DWARF 编码细节的地方
  │
  ▼
步骤 2: 变量提取
  │
  ▼
步骤 3: 类型展开
  │
  ▼
步骤 4: A2L 文本生成
```

从本步骤出来的数据，不包含任何 DWARF 编码细节（ULEB128、DWARF2 MSB 偏移等）。

---

## 二、三个子阶段

```
1a: DIE 遍历，填充原始 type_cache
 │  产出: type_cache（原始引用 + DWARF 原始 bit_offset）
 │        + global_variables + variable_types
 ▼
1b: resolve_type_graph（拓扑排序解析类型引用链）
 │  依赖 1a: 需要 type_offset 引用关系构建 DAG
 │  产出: type_cache（所有 name/size/encoding/kind 已填实）
 │        特别是 member.type_name 和 member.type_size 已解析
 ▼
1c: normalize_bitfield_offsets（位域偏移一次性规范化）
 │  依赖 1b: 需要 member.type_size 来计算存储单元大小
 │  产出: type_cache（所有 bit_offset = 绝对 LSB）
 ▼
最终输出
```

---

## 三、子阶段 1a：DIE 遍历

**输入：** .debug_info 段字节 + .debug_abbrev 段字节

**工具：** gimli 库

**过程：** gimli 遍历所有 Compilation Unit，对每个 CU 内的 DIE 按标签分发处理。

### 各标签的处理

| DIE 标签 | 处理方式 | 存入位置 |
|----------|---------|---------|
| `DW_TAG_base_type` | 从 DW_AT_name/byte_size/encoding 构建 TypeInfo::primitive() | type_cache[offset] |
| `DW_TAG_structure_type` | 创建 TypeInfo { kind: Struct }，遍历子 DIE 收集成员 | type_cache[offset] |
| `DW_TAG_union_type` | 同上，kind = Union | type_cache[offset] |
| `DW_TAG_enumeration_type` | 收集子 DIE (DW_TAG_enumerator) 的 name + value | type_cache[offset] |
| `DW_TAG_array_type` | 记录 array_dims + 元素类型的 type_offset | type_cache[offset] |
| `DW_TAG_typedef / const / volatile` | 只记录 target offset，其他字段为空或默认值 | type_cache[offset] |
| `DW_TAG_variable` | 提取 name + address + type_offset | global_variables, variable_types |

### DW_TAG_member 的处理细节

```
StructMember {
    name:         DW_AT_name（无名称则用 "_" 占位，不跳过匿名成员）
    offset:       DW_AT_data_member_location（ULEB128 解码后的绝对字节偏移）
    type_offset:  DW_AT_type（原始 DWARF 引用，1b 阶段再解析）
    type_name:    空 / 默认值（1b 阶段填充）
    type_size:    0 / 默认值（1b 阶段填充）
    bit_offset:   DWARF 原始值（1c 阶段转换）
    bit_size:     DW_AT_bit_size（位域才有）
}
```

**ULEB128 解码规则：** DW_OP_plus_uconst (opcode 0x23) 的操作数是 ULEB128 编码，每个字节低 7 位是数据，最高位为续位标志。

```
例: 0x84 0x02 → (0x04) | (0x02 << 7) = 260
```

### 1a 结束时的 type_cache 状态

- **Primitive / Struct / Union / Enum 节点：** 基本完整（name, size, members 都有）
- **成员的 type_name / type_size：** 还是空的或默认值（只有 type_offset 引用）
- **Typedef / const / volatile 节点：** 只有 target offset，其他字段全是空的
- **Array 节点：** 有维度，但元素类型只记录了 offset
- **位域成员的 bit_offset：** DWARF 原始值（DWARF2 从 MSB 算），未转换

---

## 四、子阶段 1b：resolve_type_graph

**目的：** 把 typedef → const → volatile → ... → 最终类型的引用链全部解开，让每个节点的 name/size/encoding/kind/members 都填上真实值。

**输入：** type_cache（1a 的原始输出）

### 算法

```
1. 收集所有需要解析的节点
   遍历 type_cache，找出 kind=Typedef 的节点

2. 构建依赖有向无环图（DAG）
   对每个需要解析的节点 A:
     如果 A 有 target offset → 找到节点 B
     画一条边 A → B（A 的值依赖 B 的值）

3. 拓扑排序
   保证被依赖的节点先处理
   如果排序发现环 → 报错（数据本身不应有循环引用）

4. 按拓扑序逐节点解析
   对每个节点:
     target = type_cache[target_offset]
     复制字段:
       size      = target.size
       encoding  = target.encoding
       kind      = target.kind
       members   = target.members.clone()
       name      = 保留自己的名字（见下方确认结论）

5. 第二轮：填充 StructMember 的 type_name / type_size
   遍历 type_cache 中 kind=Struct/Union 的节点
   对每个 member:
     target = type_cache[member.type_offset]
     member.type_name = target.name
     member.type_size = target.size
```

### name 字段的确认结论

typedef 节点解析后，**保留 typedef 自己的名字**（如 `LibsFltDeb_DebStatusType`），不透传目标类型的名字。

理由：
- 下游做类型推断用的是 `size` + `encoding`（通过 `infer_a2l_type_from_encoding()`），不依赖 name
- `type_name` 在 A2L 输出中只是标注字段，typedef 名比底层类型名（如 `uint16_t`）信息量更大

### 为什么拓扑排序优于迭代式

| | 当前（迭代式） | 重构后（拓扑排序） |
|---|---|---|
| 原理 | 反复遍历所有节点，每轮尝试解析 | 一次按依赖序处理 |
| 复杂度 | O(N × max_iter)，最坏 100 轮 | O(V+E)，每节点处理一次 |
| 正确性 | 长链可能不收敛；100 轮上限是拍脑袋 | 保证一次成功；环检测可报错 |

---

## 五、子阶段 1c：normalize_bitfield_offsets

**目的：** 把 DWARF 原始的 bit_offset 转为绝对 LSB 偏移，下游永远不需要知道 DWARF2/DWARF4 的区别。

**输入：** type_cache（1b 的输出，所有 type_name/type_size 已填）

**为什么依赖 1b：** 规范化公式需要 `member.type_size`（存储单元大小），而这个值在 1b 中通过解析类型引用链才填上。

### 算法

```
遍历 type_cache 中 kind=Struct/Union 的每个节点:
  遍历该节点的每个 member:
    if member.is_bitfield():

      DWARF2 格式（有 DW_AT_bit_offset）:
        存储单元大小 = member.type_size * 8

        绝对 LSB = member.offset * 8
                   + (存储单元大小 - member.bit_offset - member.bit_size)

        member.bit_offset = 绝对 LSB    // 原地替换

      DWARF4 格式（有 DW_AT_data_bit_offset）:
        member.bit_offset = data_bit_offset  // 本身就是绝对 LSB，无需转换
```

### 数学原理（DWARF2）

DWARF2 的 `DW_AT_bit_offset` 从存储单元的**最高有效位**往下数。小端模式转 LSB：

```
存储单元布局（8 位为例，MSB 在左）:
  bit: 7  6  5  4  3  2  1  0
       |<- bit_offset ->|<- bit_size ->|

LSB 位置 = type_size*8 - bit_offset - bit_size

再加上字节偏移: offset*8 + (type_size*8 - bit_offset - bit_size)
```

### 示例

**OPEN_LOAD（Cdd_L9388_SFR_RX 内部位域）：**

```
member.offset     = 1       // 字节偏移
member.type_size  = 1       // uint8 存储单元
原始 bit_offset   = 4       // DWARF2, 从 MSB 算
原始 bit_size     = 1

绝对 LSB = 1*8 + (8 - 4 - 1) = 8 + 3 = 11

替换后: member.bit_offset = 11
```

**FltDebValue（LibsFltDeb_DebHandleInfoType）：**

```
member.offset     = 0
member.type_size  = 2       // uint16 存储单元
原始 bit_offset   = ?       // DWARF2 MSB 值
原始 bit_size     = 14

绝对 LSB = 0*8 + (16 - bit_offset - 14) = 2 - bit_offset
// 具体值取决于 DWARF 原始 bit_offset
```

---

## 六、最终输出

### 产物 1：type_cache — 全局类型缓存

`HashMap<u64, TypeInfo>`，key 为 DWARF 偏移。

每个 TypeInfo 满足以下**不变量**：

| 属性 | 不变量 |
|------|--------|
| `name` | ≠ "unknown"、≠ 空；typedef 保留自身名字 |
| `size` | > 0；已从最终目标类型复制 |
| `encoding` | 已从最终目标类型复制 |
| `kind` | 已从最终目标类型复制 |

**Struct/Union 成员不变量：**

| 属性 | 不变量 |
|------|--------|
| `member.offset` | 绝对字节偏移（ULEB128 已解码） |
| `member.type_name` | 已解析的真实类型名 |
| `member.type_size` | 已解析的真实类型大小 |
| `member.bit_offset` | 如果是位域：绝对 LSB 位偏移（从结构体起始字节算，bit 0 = byte 0 的 bit 0） |
| `member.bit_size` | 如果是位域：位宽度 |
| `member.name` | 匿名成员用 "_" 占位，不丢失 |

### 产物 2：global_variables

```
Vec<DwarfVariable> {
    name: "Cdd_L9388_SFR_RX",        // ELF 全局变量名
    address: 0x70027000,              // 绝对内存地址
    type_offset: 0x2847,              // → type_cache[0x2847] 拿到完整 TypeInfo
}
```

### 产物 3：variable_types

`HashMap<String, u64>` — 变量名 → type_cache key 的快速查找表。

与 global_variables 是同一份数据的另一种索引方式。

### type_cache 中间节点处理确认

typedef/const/volatile 等中间节点**保留在 type_cache 中**，内容已被拉平为目标类型的值。

理由：下游很多地方用 type_offset 随机查找，删除中间节点会导致查找失败。

---

## 七、具体示例：type_cache 中一条 struct 记录

```rust
// typedef struct { uint16 FltDebValue:14; LibsFltDeb_DebStatusType CurrentDebStatus:2; }
// LibsFltDeb_DebHandleInfoType;

type_cache[0x1523] = TypeInfo {
    name: "LibsFltDeb_DebHandleInfoType",
    size: 2,
    kind: Struct,
    encoding: Unsigned,
    members: [
        StructMember {
            name: "FltDebValue",
            offset: 0,
            type_name: "uint16",
            type_size: 2,
            bit_offset: Some(0),    // 绝对 LSB: byte 0, bit 0
            bit_size: Some(14),
        },
        StructMember {
            name: "CurrentDebStatus",
            offset: 0,              // 同一容器，字节偏移相同
            type_name: "LibsFltDeb_DebStatusType",  // typedef 名，非 uint8
            type_size: 2,           // 已从目标类型复制（uint16 的 size）
            bit_offset: Some(14),   // 绝对 LSB: 紧接上一个位域
            bit_size: Some(2),
        },
    ],
    variants: [],
    array_dims: [],
    pointer_target: None,
    offset: 0x1523,
}
```

---

## 八、讨论确认记录

- [x] 子阶段 1c 依赖 1b（需要 type_size 计算存储单元大小）→ 执行顺序确认
- [x] typedef 节点的 name 保留自身名字，size/encoding/kind 从目标类型复制
- [x] 中间类型节点保留在 type_cache 中，内容拉平为目标类型的值
- [x] 匿名成员用 "_" 占位，不跳过
- [ ] DWARF4 的 DW_AT_data_bit_offset 是否需要支持（待确认）
