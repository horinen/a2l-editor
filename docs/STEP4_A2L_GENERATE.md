# 步骤 4：A2L 文本生成

> 状态：已讨论确认
> 模块：a2l.rs
> 定位：把 A2lEntryStore 格式化为 A2L 文本

---

## 一、在管线中的位置

```
步骤 3 输出: A2lEntryStore
  │
  ▼
步骤 4: A2L 文本生成（本步骤）
  │  输入: A2lEntryStore + endianness
  │  输出: A2L 文本字符串
```

---

## 二、前置条件

| 保证 | 来源 |
|------|------|
| A2lEntry.bit_offset 对位域变量已是绝对 LSB 偏移 | 步骤 1c → 步骤 3 透传 |
| A2lEntry.symbol_link_name/offset 对位域变量已正确设置 | 步骤 3 expand_bitfield |
| 非位域变量的 bit_offset/bit_size/symbol_link 均为 None | 步骤 3 只为位域设置这些字段 |

---

## 三、核心设计原则

步骤 4 是**几乎纯格式化器**——A2lEntry → A2L 文本。唯一的非格式化操作是 BIT_MASK 计算，需要 endianness 参数。

### 删除 get_effective_bit_offset()

当前代码中 `get_effective_bit_offset(endianness)` 的实现直接返回 `self.bit_offset`（之前的 bug 已修复），但函数签名暗示"这里有转换"，给调用者错觉。

重构后删除此函数，直接用 `entry.bit_offset`。

---

## 四、endianness 的处理位置

### 为什么 endianness 留在步骤 4

endianness 本质上是 **A2L 输出格式的属性**，不是 DWARF 数据的属性，也不是类型结构的属性。

- StructMember.bit_offset（步骤 1c 输出）：绝对 LSB 位偏移，字节序无关
- A2lEntry.bit_offset（步骤 3 输出）：绝对 LSB 位偏移，字节序无关
- BIT_MASK（步骤 4 输出）：A2L 格式特定，字节序相关

### BIT_MASK 计算与字节序的关系

```
32 位容器，bit_offset=11, bit_size=1

物理含义: 容器 byte[1] 的 bit 3（从 byte 0 bit 0 数起的第 11 位）

小端（byte[0] = LSB）:
  读取的 uint32 value 中，第 11 位就是这个物理位
  mask = ((1 << 1) - 1) << 11 = 0x800

大端（byte[0] = MSB）:
  读取的 uint32 value 中，第 20 位才对应这个物理位
  mask = ((1 << 1) - 1) << (32 - 11 - 1) = 0x100000
```

### calculate_bit_mask 的重构

```rust
fn calculate_bit_mask(bit_offset: usize, bit_size: usize, container_size_bits: usize, endianness: Endianness) -> u64 {
    let shift = match endianness {
        Endianness::Little => bit_offset,
        Endianness::Big => container_size_bits - bit_offset - bit_size,
    };
    ((1u64 << bit_size) - 1) << shift
}
```

参数来源：
- `bit_offset` = `entry.bit_offset.unwrap()`
- `bit_size` = `entry.bit_size.unwrap()`
- `container_size_bits` = `entry.size * 8`（位域 entry 的 size 就是容器大小）
- `endianness` = 用户设置

---

## 五、两个生成函数

### generate_measurement_block

```
输入: entry, compu_method, endianness
输出: MEASUREMENT block 文本

过程:
  1. a2l_type = entry.a2l_type
  2. format_str = get_format_string(a2l_type)
  3. min/max:
     bitfield → (0, (1 << bit_size) - 1)
     其他    → get_min_max(a2l_type)
  4. 输出头: /begin MEASUREMENT {full_name} ""
  5. 输出类型行: {a2l_type} {compu} 0 0 {min} {max}
  6. bitfield → 输出 BIT_MASK:
     mask = calculate_bit_mask(bit_offset, bit_size, size*8, endianness)
  7. 输出 ECU_ADDRESS 0x{address:08X}
  8. 输出 ECU_ADDRESS_EXTENSION 0x0
  9. 输出 FORMAT "{format_str}"
  10. SYMBOL_LINK:
      bitfield + symbol_link_name → SYMBOL_LINK "{symbol_link_name}" {symbol_link_offset}
      其他                       → SYMBOL_LINK "{full_name}" 0
  11. 输出 /end MEASUREMENT
```

### generate_characteristic_block

```
输入: entry, compu_method, endianness
输出: CHARACTERISTIC block 文本

过程与 measurement 类似，区别:
  - 类型行格式: VALUE 0x{address:08X} {record_layout} {compu} 0 {max} 0 {max}
  - 输出 EXTENDED_LIMITS 0 {max}
  - BIT_MASK 和 SYMBOL_LINK 逻辑完全相同
```

---

## 六、不动的部分

以下函数和逻辑不在重构范围内，保持当前实现：

| 函数/逻辑 | 理由 |
|-----------|------|
| `get_format_string` | 纯查表 |
| `get_min_max` | 纯查表 |
| `get_record_layout` | 纯查表 |
| `get_bitfield_max` | 纯计算 `(1 << bit_size) - 1` |
| `generate_compu_method_name/block` | COMPU_METHOD 生成，与 bit_offset 无关 |
| `parse_existing_names` | 解析已有 A2L，不涉及 bit_offset |
| `append_to_file` | 文件操作 + 去重逻辑 |
| `remove_variables` / `modify_variable` / `apply_changes` | A2L 编辑功能，独立于生成逻辑 |
| `A2lParser` | A2L 文件解析，独立于生成逻辑 |

---

## 七、与当前代码的 diff 概要

| 变化 | 说明 |
|------|------|
| 删除 `A2lEntry::get_effective_bit_offset()` | bit_offset 已是绝对 LSB，直接使用 |
| 删除 `StructMember::get_effective_bit_offset()` | 同上 |
| `calculate_bit_mask` 增加 `container_size_bits` 和 `endianness` 参数 | 支持大端 |
| `generate_measurement_block_with_compu` 删除 `symbol_link` 参数 | SYMBOL_LINK 信息已在 A2lEntry 上，不需要外部传入 |
| `generate_characteristic_block_with_compu` 同上 | 同上 |

### symbol_link 参数的删除

当前函数签名：
```rust
fn generate_measurement_block_with_compu(
    entry: &A2lEntry,
    compu_method: Option<&str>,
    symbol_link: Option<&str>,   // ← 删除
    endianness: Endianness,
)
```

删除理由：SYMBOL_LINK 的所有信息已在 A2lEntry 上（`symbol_link_name`, `symbol_link_offset`），不需要外部参数覆盖。当前调用点都传 `None`，只有 `apply_changes` 的编辑路径会用，编辑路径应直接操作文本而非通过生成函数。

---

## 八、A2L 输出示例

### 普通变量（小端）

```
/begin MEASUREMENT Cdd_L9388_SFR_RX.WDG2TOUTTMG ""
  ULONG NO_COMPU_METHOD 0 0 0 4294967295
  ECU_ADDRESS 0x070027100
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%10.0"
  SYMBOL_LINK "Cdd_L9388_SFR_RX.WDG2TOUTTMG" 0
/end MEASUREMENT
```

### 位域变量（小端）

```
/begin MEASUREMENT Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD ""
  ULONG NO_COMPU_METHOD 0 0 0 1
  BIT_MASK 0x800
  ECU_ADDRESS 0x070027AAC
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%10.0"
  SYMBOL_LINK "Cdd_L9388_SFR_RX" 684
/end MEASUREMENT
```

BIT_MASK 计算：
```
bit_offset=11, bit_size=1, container_size=4B=32bit, Little
mask = ((1 << 1) - 1) << 11 = 0x800
```

### 位域变量（大端，同一个物理位）

```
/begin MEASUREMENT Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD ""
  ULONG NO_COMPU_METHOD 0 0 0 1
  BIT_MASK 0x100000
  ECU_ADDRESS 0x070027AAC
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%10.0"
  SYMBOL_LINK "Cdd_L9388_SFR_RX" 684
/end MEASUREMENT
```

BIT_MASK 计算：
```
bit_offset=11, bit_size=1, container_size=4B=32bit, Big
shift = 32 - 11 - 1 = 20
mask = ((1 << 1) - 1) << 20 = 0x100000
```

---

## 九、讨论确认记录

- [x] 删除 `get_effective_bit_offset()`（A2lEntry 和 StructMember 上的都删）
- [x] endianness 保留在步骤 4，只影响 `calculate_bit_mask`
- [x] `calculate_bit_mask` 增加 `container_size_bits` 和 `endianness` 参数
- [x] A2lEntry.bit_offset 和 StructMember.bit_offset 保持"绝对 LSB"语义，字节序无关
- [x] 删除 `generate_*_block` 函数的 `symbol_link` 外部参数（信息已在 entry 上）
- [x] 步骤 1-3 的测试不需要参数化 endianness
- [x] 不支持大端时不影响步骤 1-3 的逻辑
