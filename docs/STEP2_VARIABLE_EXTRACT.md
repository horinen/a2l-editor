# 步骤 2：变量提取

> 状态：已讨论确认
> 模块：elf.rs
> 定位：把步骤 1 的 DWARF 维度数据转换为步骤 3 可消费的变量维度数据

---

## 一、在管线中的位置

```
步骤 1 输出: type_cache + global_variables + variable_types
  │
  ▼
步骤 2: 变量提取（本步骤）
  │  输入: DwarfParser 三件产物
  │  输出: Vec<Variable>（每个携带完整 TypeInfo）
  ▼
步骤 3: 类型展开
```

---

## 二、前置条件（步骤 1 的保证）

步骤 2 能正确工作的前提：

| 保证 | 来源 | 失败意味着 |
|------|------|-----------|
| type_cache 中所有节点的 name/size 已填实 | 步骤 1b resolve_type_graph | TypeInfo.size=0，变量被误丢弃 |
| type_cache 中所有 bit_offset 已规范化 | 步骤 1c normalize_bitfield_offsets | 位域变量携带错误的偏移进入步骤 3 |
| 不存在 type_offset 指向 type_cache 中不存在条目的情况 | 步骤 1a DIE 遍历的完整性 | 查找失败 |

如果前置条件不满足，说明步骤 1 有 bug，步骤 2 应报错而非静默跳过。

---

## 三、实现原理

### 核心逻辑

步骤 2 做一件事：**把 DWARF 维度的数据转换为变量维度**。

- 步骤 1 的维度是**类型**（type_cache 按 DWARF 偏移索引，global_variables 是附带产物）
- 步骤 3 的维度是**变量**（逐个展开变量的类型树）

步骤 2 的核心操作就是把这两个维度接起来：`DwarfVariable.type_offset → type_cache 查找 → Variable.type_info`。

### 算法

```
输入:
  global_variables: [
    DwarfVariable { name: "Cdd_L9388_SFR_RX", address: 0x70027000, type_offset: 0x2847 }
    DwarfVariable { name: "Cdd_L9388_FaultDebInfo", address: 0x70027300, type_offset: 0x15A0 }
    ...
  ]

  type_cache: {
    0x2847 → TypeInfo { name: "Cdd_L9388_Register_Type", size: 620, kind: Struct, ... }
    0x15A0 → TypeInfo { name: "...", size: 12, kind: Array, ... }
    ...
 }

处理过程:
  对每个 DwarfVariable:
    1. 用 type_offset 从 type_cache 查找 TypeInfo
    2. 查到 + size > 0 → 构建 Variable
    3. 查不到 → 报错（步骤 1 的保证被违反）
    4. 去重（同名变量只保留第一个）
    5. 排序（按 name 字典序）

输出:
  Vec<Variable> [
    Variable { name: "Cdd_L9388_FaultDebInfo", address: 0x70027300, size: 12, ... },
    Variable { name: "Cdd_L9388_SFR_RX", address: 0x70027000, size: 620, ... },
    ...
  ]
```

### 与当前代码的区别

| | 当前 | 重构后 |
|---|---|---|
| 路径数 | 三条（DWARF/ELF符号表/enrich补全） | 一条（仅 DWARF，无 DWARF 则报错） |
| enrich 步骤 | extract 后单独跑一轮 enrich 补全 | 合并进 extract，一轮完成 |
| type_info | Option\<TypeInfo\> | TypeInfo（必选） |
| section 字段 | 存在，DWARF 路径下为空字符串 | 删除（仅 CLI 统计用，非核心管线） |
| 查找失败 | 静默跳过 | 报错 |

---

## 四、输出产物

### Variable 结构体（重构后）

```rust
pub struct Variable {
    pub name: String,           // 变量名，唯一
    pub address: u64,           // 绝对内存地址
    pub size: usize,            // 字节大小 = type_info.size
    pub type_name: String,      // 类型名 = type_info.name
    pub type_info: TypeInfo,    // 完整的、已规范化的类型信息（必选）
}
```

### 变化说明

| 字段 | 变化 | 理由 |
|------|------|------|
| `section` | **删除** | 仅 CLI 统计使用，非核心管线；DWARF 路径下永远为空 |
| `type_info` | Option\<TypeInfo\> → **TypeInfo** | 只考虑有 DWARF 的情况，无类型信息则报错 |

### 输出不变量

对 Vec\<Variable\> 中的每个 Variable：

| 属性 | 不变量 |
|------|--------|
| `name` | 唯一（已去重），非空 |
| `address` | > 0 |
| `size` | > 0，等于 `type_info.size` |
| `type_name` | 等于 `type_info.name` |
| `type_info` | 满足步骤 1 输出的所有不变量（bit_offset 已规范化，type_name/type_size 已填实） |

### 连锁影响

`type_info` 从 `Option<TypeInfo>` 变为 `TypeInfo`，步骤 3 的 `expand_variable` 入口不再需要处理 `type_info = None` 的分支，直接从 `var.type_info` 开始递归展开。

---

## 五、边界情况

| 情况 | 处理 |
|------|------|
| 同名变量出现多次 | 只保留第一个（保持当前行为） |
| DwarfVariable.type_offset 在 type_cache 中不存在 | **报错**（步骤 1 保证被违反） |
| global_variables 为空 | 返回空 Vec，不报错（ELF 可能确实没有导出变量） |
| 无 DWARF 信息 | **报错**（不支持退化路径） |

---

## 六、具体示例

### 输入（步骤 1 输出）

```
global_variables: [
    DwarfVariable { name: "Cdd_L9388_SFR_RX", address: 0x70027000, type_offset: 0x2847 },
    DwarfVariable { name: "Cdd_L9388_FaultDebInfo", address: 0x70027300, type_offset: 0x15A0 },
]

type_cache: {
    0x2847 → TypeInfo { name: "Cdd_L9388_Register_Type", size: 620, kind: Struct, ... },
    0x15A0 → TypeInfo { name: "Cdd_L9388_FaultDebArrayType", size: 12, kind: Array, ... },
}
```

### 输出（步骤 2 输出）

```rust
vec![
    Variable {
        name: "Cdd_L9388_FaultDebInfo",
        address: 0x70027300,
        size: 12,
        type_name: "Cdd_L9388_FaultDebArrayType",
        type_info: TypeInfo { /* 完整的 Array 类型，包含元素类型、维度等 */ },
    },
    Variable {
        name: "Cdd_L9388_SFR_RX",
        address: 0x70027000,
        size: 620,
        type_name: "Cdd_L9388_Register_Type",
        type_info: TypeInfo { /* 完整的 Struct 类型，包含所有成员、已规范化的 bit_offset */ },
    },
]
```

（按 name 字典序排列）

---

## 七、讨论确认记录

- [x] 只考虑有 DWARF 输出的情况，无 DWARF 直接报错
- [x] 合并 extract + enrich 为一步（步骤 1 的 type_cache 已完全解析，不需要两轮）
- [x] 删除 `section` 字段（仅 CLI 统计使用，DWARF 路径下永远为空）
- [x] `type_info` 从 Option\<TypeInfo\> 改为 TypeInfo（必选）
- [x] type_offset 查找失败时报错，不静默跳过
