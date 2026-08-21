# 问题追踪

## [FIXED] array[1] 结构体成员未展开

**发现日期**: 2026-03-21
**修复日期**: 2026-03-21
**严重程度**: 中

### 问题描述

当变量类型为 `array[1]` 且元素为匿名结构体时，结构体成员未被展开到 A2L 条目中。

### 根本原因

1. **匿名成员被跳过**: `parse_member` 函数在成员无名称时返回 `None`，导致匿名联合体/结构体成员被完全跳过。
2. **resolve_all_member_types 只遍历 struct_map**: 匿名结构体不存入 `struct_map`，导致其成员的类型名称未被解析。

### 修复方案

**修复 1**: `src/lib/dwarf.rs:451` - 为无名成员生成占位名称
```rust
// 修复前
let name = Self::get_name_static(entry)?;

// 修复后
let name = Self::get_name_static(entry).unwrap_or_else(|| "_".to_string());
```

**修复 2**: `src/lib/dwarf.rs:252-290` - 遍历 `type_cache` 而非 `struct_map`
```rust
// 修复前：只遍历 struct_map（不含匿名结构体）
for type_info in self.struct_map.values_mut() { ... }

// 修复后：遍历 type_cache 中的所有结构体/联合体
let offsets: Vec<u64> = self.type_cache.keys().copied().collect();
for offset in offsets {
    if let Some(type_info) = self.type_cache.get_mut(&offset) {
        if type_info.kind == TypeKind::Struct || type_info.kind == TypeKind::Union {
            // 解析成员类型...
        }
    }
}
```

### 验证结果

```
Cdd_TLE918X_SFR_RX: 匹配条目数从 1 增加到 670
Cdd_TLE918X_SFR_RX._0_._.REGISTER.CONF_SIG
Cdd_TLE918X_SFR_RX._0_._.REGISTER.CONF_GEN_1
...（所有寄存器成员正确展开）
```

---

## [FIXED] 位域成员的地址、大小和类型推断错误

**发现日期**: 2026-04-01
**修复日期**: 2026-04-01
**严重程度**: 高

### 问题描述

结构体中的位域（bitfield）成员在展开为 A2L 条目时，地址、字节大小和 A2L 类型不正确。

**复现变量**: `Cdd_L9388_FaultDebInfo`，其元素类型为：
```c
typedef struct {
    uint16 FltDebValue       : 14;  // 14 位
    LibsFltDeb_DebStatusType CurrentDebStatus : 2;   // 2 位
} LibsFltDeb_DebHandleInfoType;
```

两个位域共享同一个 `uint16`（2 字节）容器。

**实际输出**:
```
Cdd_L9388_FaultDebInfo._0_.FltDebValue      @ 0x7002734C   2B UWORD bits[2,15]
Cdd_L9388_FaultDebInfo._0_.CurrentDebStatus  @ 0x7002734D   1B UBYTE  bits[0,1]
```

**预期输出**:
```
Cdd_L9388_FaultDebInfo._0_.FltDebValue      @ 0x7002734C   2B UWORD bits[2,15]
Cdd_L9388_FaultDebInfo._0_.CurrentDebStatus  @ 0x7002734C   2B UWORD  bits[0,1]
```

### 错误详情

| 属性 | CurrentDebStatus 实际 | 预期 | 说明 |
|------|----------------------|------|------|
| 地址 | `0x7002734D` (+1) | `0x7002734C` | 同一容器的位域应共享容器起始地址 |
| 大小 | `1B` | `2B` | 位域 size 应为容器大小，非字段类型大小 |
| 类型 | `UBYTE` | `UWORD` | 由容器大小（2B）推断，非字段类型大小（1B） |

### 根本原因

`src/lib/elf.rs` `expand_recursive` 函数中，位域成员直接使用 `member.offset` 和 `member.type_size`：
- `member.offset` 是 DWARF 中该成员的 `DW_AT_data_member_location`，对于共享容器的位域，不同成员可能有不同的字节偏移（如 FltDebValue=0, CurrentDebStatus=1），但它们共享同一个容器
- `member.type_size` 是该位域字段自身类型的大小（如 `CurrentDebStatus` 的枚举类型可能只有 1 字节），而非容器（`uint16` = 2 字节）的大小

A2L 规范要求位域的记录单元（BIT_OPERATION 或 MASK）基于整个容器，因此必须使用容器大小和容器起始地址。

### 修复方案

在 `expand_recursive` 的 Struct/Union 分支中，先计算位域容器分组，再展开：

**核心逻辑**: 新增 `compute_bitfield_groups` 函数，识别连续位域成员并计算容器信息：
1. 遍历成员列表，将连续的位域成员分为一组
2. 同组内取最小 `offset` 作为容器起始偏移
3. 同组内取最大 `type_size` 作为容器大小

```rust
fn compute_bitfield_groups(members: &[StructMember]) -> HashMap<usize, (usize, usize)> {
    // 连续位域为一组，同一容器
    // key: member.offset -> value: (容器起始offset, 容器size)
}
```

展开时使用容器信息：
```rust
if member.is_bitfield() {
    let &(container_offset, container_size) = bitfield_groups.get(&member.offset);
    let container_addr = base_addr + container_offset as u64;
    let a2l_type = infer_a2l_type_from_encoding(container_size, ...);
    // 使用 container_addr 和 container_size 创建 A2lEntry
}
```

### 相关文件

- `src/lib/elf.rs` - `expand_recursive`, `compute_bitfield_groups`
- `src/lib/dwarf.rs` - `parse_member`, `get_bitfield_info_static`
- `src/lib/types.rs` - `StructMember`, `A2lEntry`

---

## [OPEN] DW_AT_data_member_location 的 ULEB128 解码错误，导致结构体偏移 >= 256 的成员偏移量全部错误

**发现日期**: 2026-04-02
**严重程度**: 高

### 问题描述

`Cdd_L9388_SFR_RX` 变量类型为 `Cdd_L9388_Register_Type`（620 字节），DWARF 解析出的结构体成员偏移从 offset >= 256 开始全部错误。

**复现步骤**:
```bash
cargo run --bin a2l-cli -- type temp/test.elf Cdd_L9388_SFR_RX
```

**实际输出 vs 正确偏移（readelf -wi 验证）**:

| 成员 | 实际偏移 | 正确偏移 | DWARF LEB128 编码 |
|------|---------|---------|-------------------|
| WDG2TOUTTMG | +128 | +256 | `0x80 0x02` |
| WDG2PGM | +132 | +260 | `0x84 0x02` |
| WDG2ANS | +136 | +264 | `0x88 0x02` |
| CHx_FAULT | +140 | +268 | `0x8c 0x02` |
| EXCEPTIONS_CH0_5 | +164 | +292 | `0xa4 0x02` |
| PWMSENSE_CH6_7 | +228 | +612 | `0xe4 0x04` |

### 根本原因

`src/lib/dwarf.rs` 第 808-842 行 `get_member_location_static` 函数处理 `DW_AT_data_member_location` 时，`DW_OP_plus_uconst`（opcode `0x23`）的操作数是 **ULEB128 编码**，但代码将其当作原始字节拼接：

```rust
// 当前错误代码 (Block 分支，Exprloc 分支同理)
if block.len() >= 2 && block[0] == 0x23 {
    Some(block[1] as usize)              // 只读首字节，>= 128 就错
} else if block.len() >= 3 && block[0] == 0x23 {
    Some(block[1] as usize | ((block[2] as usize) << 8))  // 小端拼接，非 ULEB128
}
```

ULEB128 解码规则：每个字节的低 7 位是数据，最高位为续位标志。例如 `0x84 0x02` = `(0x04) | (0x02 << 7)` = 260，而非 `0x84 | (0x02 << 8)` = 644。

偏移 < 128 的成员（LEB128 单字节编码，值等于原始字节）不受影响。偏移 128-255 的成员因 LEB128 双字节编码中第二字节为 `0x01`，错误解码碰巧得到正确结果。偏移 >= 256 开始暴露。

### 修复方案

添加 ULEB128 解码函数，替换 `Block` 和 `Exprloc` 分支中的手动字节拼接：

```rust
fn read_uleb128(data: &[u8]) -> usize {
    let mut result = 0usize;
    let mut shift = 0;
    for &byte in data {
        result |= ((byte & 0x7f) as usize) << shift;
        if byte & 0x80 == 0 { break; }
        shift += 7;
    }
    result
}
```

```rust
Block(block) | Exprloc(expr) => {
    let data = &block[..]; // 或 &expr.0[..]
    if data.len() >= 2 && data[0] == 0x23 {
        Some(read_uleb128(&data[1..]))
    } else {
        None
    }
}
```

### 影响范围

所有包含成员偏移 >= 128 的结构体类型，其成员偏移量解析均可能受影响。偏移 < 128 的成员不受影响。

### 相关文件

- `src/lib/dwarf.rs` - `get_member_location_static` 函数（第 808-842 行）

---

## [OPEN] bitfield 变量 SYMBOL_LINK 指向点分路径，CANape 更新地址后覆盖为错误值

**发现日期**: 2026-04-03
**严重程度**: 高

### 问题描述

A2L 文件中的 bitfield 变量（使用 BIT_MASK 的 MEASUREMENT）导入 CANape 后，CANape 的符号更新机制会将 `ECU_ADDRESS` 覆盖为错误的地址。

**复现变量**: `Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD`

结构体定义（小端模式）：
```c
union {
    uint32 R;
    struct {
        uint32 Crc          :5;   // byte 0, bits[0..4]
        uint32 LS_CLAMP_ON  :1;   // byte 0, bit 5
        uint32 ...
        uint32 OPEN_LOAD    :1;   // byte 1, bit 11
        uint32 ...
    } B;
} EXCEPTIONS_CH0_5[6];
```

**当前 A2L 输出**:
```
/begin MEASUREMENT Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD ""
  ULONG NO_COMPU_METHOD 0 0 0 1
  BIT_MASK 0x800
  ECU_ADDRESS 0x70027AAC                    ← 正确的容器地址
  SYMBOL_LINK "Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD" 0
/end MEASUREMENT
```

**CANape 更新后的错误地址**:
```
ECU_ADDRESS 0x70027AAD                      ← 错误！CANape 按 SYMBOL_LINK 路径计算出 byte_offset=1
```

OPEN_LOAD 的 DWARF `data_member_location` 是 1（在 byte 1），CANape 沿点分路径解析结构体时得到字节偏移 1，将容器地址 `0x70027AAC` 覆盖为 `0x70027AAC + 1 = 0x70027AAD`。但 bitfield 的 ECU_ADDRESS 必须指向容器起始地址（byte 0），BIT_MASK 才能正确工作。

### 根本原因

A2L 规范中 `SYMBOL_LINK` 的格式为：
```
SYMBOL_LINK "<符号名>" <字节偏移>
```

当前代码（`src/lib/a2l.rs:202,224`）对所有变量统一使用点分路径名 + 偏移 0：
```rust
let link = symbol_link.unwrap_or(&entry.full_name);
output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", link));
```

对于 bitfield 变量，`entry.full_name` 是类似 `"Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD"` 的点分路径。CANape 解析此路径时会沿结构体层级逐级计算字节偏移，最终得到 bitfield 成员在容器内的字节偏移（如 byte 1），而非容器本身的起始偏移（byte 0）。

正确做法是让 SYMBOL_LINK 指向 ELF 中的真实符号（顶层变量名），并通过偏移量指向容器位置：
```
SYMBOL_LINK "Cdd_L9388_SFR_RX" <容器在结构体中的字节偏移>
```

CANape 的更新机制会计算：`ELF 符号地址 + SYMBOL_LINK 偏移 = 容器地址`，与 BIT_MASK 配合正确解析 bitfield。

### 修复方案

#### 修改 1: `src/lib/types.rs` — A2lEntry 添加 SYMBOL_LINK 元信息

```rust
pub struct A2lEntry {
    // ... 现有字段 ...
    pub symbol_link_name: Option<String>,   // ELF 根符号名（如 "Cdd_L9388_SFR_RX"）
    pub symbol_link_offset: Option<u64>,    // 容器在根符号中的字节偏移
}
```

仅 bitfield 变量需要设置这两个字段，非 bitfield 变量保持 `None`，仍使用 `full_name` + `0`。

#### 修改 2: `src/lib/elf.rs` — expand_recursive 传递根符号信息

`expand_recursive` 签名新增参数 `root_symbol: Option<&str>` 和 `root_addr: Option<u64>`：

- `expand_variable`（入口）：传入 `Some(&var.name)`, `Some(var.address)`
- bitfield 条目创建处：计算 `symbol_link_offset = container_addr - root_addr`
  ```rust
  store.add(
      A2lEntry::new(...)
          .with_bitfield(actual_bit_offset, raw_bs)
          .with_symbol_link(root_symbol.unwrap().to_string(), container_addr - root_addr.unwrap()),
  );
  ```
- 递归调用（`expand_recursive`, `expand_multi_dim_array`）：透传 `root_symbol`, `root_addr`

#### 修改 3: `src/lib/a2l.rs` — SYMBOL_LINK 生成逻辑

在 `generate_measurement_block_with_compu` 和 `generate_characteristic_block_with_compu` 中：

```rust
if entry.is_bitfield() && entry.symbol_link_name.is_some() {
    let name = entry.symbol_link_name.as_ref().unwrap();
    let offset = entry.symbol_link_offset.unwrap_or(0);
    output.push_str(&format!("      SYMBOL_LINK \"{}\" {}\n", name, offset));
} else {
    let link = symbol_link.unwrap_or(&entry.full_name);
    output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", link));
}
```

#### 修改 4: IPC 层（可选）

`EntryInfo`（`src-tauri/src/commands.rs:62`）已有 `symbol_link: Option<String>` 字段，可在 `From` 实现中将 bitfield 的 symbol_link 组合为 `"name offset"` 格式传递给前端。如果前端不直接处理 SYMBOL_LINK，此步可省略。

### 预期效果

```
// 修复前
SYMBOL_LINK "Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD" 0
→ CANape 计算出 base + 1（错误）

// 修复后
SYMBOL_LINK "Cdd_L9388_SFR_RX" <容器偏移>
→ CANape 计算出 base + 容器偏移（正确，与 ECU_ADDRESS 一致）
```

### 影响范围

所有使用 BIT_MASK 的 bitfield MEASUREMENT/CHARACTERISTIC 变量。非 bitfield 变量不受影响。

### 相关文件

- `src/lib/types.rs` — `A2lEntry` 结构体（第 378-428 行）
- `src/lib/elf.rs` — `expand_recursive`（第 336-476 行）、`expand_variable`（第 306-334 行）、`expand_multi_dim_array`（第 562-617 行）
- `src/lib/a2l.rs` — `generate_measurement_block_with_compu`（第 184-228 行）、`generate_characteristic_block_with_compu`（第 230-272 行）
- `src-tauri/src/commands.rs` — `EntryInfo`（第 62-88 行）

---

## [OPEN] DWARF2 位域偏移计算错误 — DW_AT_bit_offset 未按存储单元转换

**发现日期**: 2026-04-02
**严重程度**: 高

### 问题描述

结构体中的位域（bitfield）成员，当其 `DW_AT_data_member_location`（字节偏移）不为 0 时，解析出的 LSB 位偏移不正确。

**复现变量**: `Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD`

**C 头文件定义** (`Cdd_L9388_Types.h:1887-1924`，小端模式):
```c
union {
    uint32 R;
    struct {
        uint32 Crc          :5;   // byte 0, bits[0..4]
        uint32 LS_CLAMP_ON  :1;   // byte 0, bit 5
        uint32 LS_OVC       :1;   // byte 0, bit 6
        uint32 VGS_LS_FAULT :1;   // byte 0, bit 7
        uint32 VGS_HS_FAULT :1;   // byte 1, bit 8
        uint32 HS_SHORT     :1;   // byte 1, bit 9
        uint32 LVT          :1;   // byte 1, bit 10
        uint32 OPEN_LOAD    :1;   // byte 1, bit 11
        uint32 GND_LOSS     :1;   // byte 1, bit 12
        uint32              :1;   // byte 1, bit 13
        uint32 TH_WARN      :1;   // byte 1, bit 14
        uint32 T_SD         :1;   // byte 1, bit 15
        uint32              :5;   // byte 2
        uint32 Cmd          :11;  // byte 2-3
    } B;
} EXCEPTIONS_CH0_5[6];
```

**实际输出**:
```
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD  @ 0x70027AAC  4B ULONG bits[4,4]
```
bit_offset=4, bit_size=1 → BIT_MASK = `0x10`

**预期输出**:
```
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD  @ 0x70027AAC  4B ULONG bits[11,11]
```
bit_offset=11, bit_size=1 → BIT_MASK = `0x800`

### 错误详情

同一结构体（`EXCEPTIONS_CH0_5[0].B`）所有位域成员对比：

| 成员 | DWARF byte_offset | DWARF bit_offset | 期望 LSB | 实际 LSB |
|------|-------------------|-----------------|---------|---------|
| Crc (5bit) | 0 | 3 | 0 | 3 |
| LS_CLAMP_ON | 0 | 2 | 5 | 2 |
| VGS_LS_FAULT | 0 | 0 | 7 | 0 |
| VGS_HS_FAULT | 1 | 7 | 8 | 7 |
| OPEN_LOAD | 1 | 4 | **11** | **4** |
| Cmd (11bit) | 2 | 0 | 21 | 0 |

byte_offset=0 的成员碰巧偏移错误但范围在 0-7 内不溢出；byte_offset>0 的成员偏移完全错误。

### 根本原因

DWARF2 中 `DW_AT_bit_offset` 是从**存储单元（storage unit，由 `DW_AT_byte_size` 决定）MSB** 开始的偏移，而非从整个容器 MSB 的偏移。当前代码有 **两处转换错误**：

#### 错误 1: `src/lib/elf.rs:378-397` — 未将 DWARF bit_offset 转为绝对 LSB 偏移

创建 bitfield A2lEntry 时，直接将 DWARF 的 `DW_AT_bit_offset` 存入 `A2lEntry.bit_offset`，没有考虑成员在结构体中的字节偏移：

```rust
// 当前代码 - 直接存储 DWARF 原始值
.with_bitfield(
    member.bit_offset.unwrap_or(0),   // DWARF 的 DW_AT_bit_offset，未经转换
    member.bit_size.unwrap_or(0),
)
```

正确计算需要结合 `DW_AT_data_member_location`（字节偏移）和 `DW_AT_byte_size`（存储单元大小）：

```
绝对 LSB 偏移 = byte_in_container * 8 + (storage_unit_bits - DW_AT_bit_offset - DW_AT_bit_size)
```

以 OPEN_LOAD 为例：
```
byte_in_container = 1 (data_member_location=1, container_start=0)
storage_unit_bits = 8  (DW_AT_byte_size=1)
绝对偏移 = 1 * 8 + (8 - 4 - 1) = 11 ✓
```

#### 错误 2: `src/lib/types.rs:427-434` — get_effective_bit_offset 使用容器大小转换

```rust
// 当前代码 - 用整个容器（32位）做转换，得到错误结果
pub fn get_effective_bit_offset(&self, endianness: Endianness) -> Option<usize> {
    let raw_offset = self.bit_offset?;
    let bit_size = self.bit_size?;
    let container_size_bits = self.size * 8;   // 32 (整个 union 容器)
    let effective = match endianness {
        Endianness::Little => container_size_bits - raw_offset - bit_size,  // 32 - 4 - 1 = 27 ✗
        Endianness::Big => raw_offset,
    };
    Some(effective)
}
```

如果 A2lEntry.bit_offset 存的已经是转换后的 LSB 绝对偏移，此函数不应再做转换。目前由于 bit_offset 存的是未转换值，此函数又基于容器大小做了一次错误转换，两者叠加导致结果不正确。

### DWARF 编码背景

DWARF 标准对位域有两种编码方式：

| DWARF 版本 | 位偏移属性 | 含义 |
|------------|-----------|------|
| DWARF 2/3 | `DW_AT_bit_offset` + `DW_AT_bit_size` + `DW_AT_byte_size` | bit_offset 从**存储单元** MSB 算起，byte_size 定义存储单元大小 |
| DWARF 4+ | `DW_AT_data_bit_offset` + `DW_AT_bit_size` | data_bit_offset 是从结构体起始的**绝对 LSB 位偏移**，无需转换 |

当前 ELF（GCC 产生的 DWARF2）使用前者，需要结合 `DW_AT_data_member_location`、`DW_AT_byte_size`、`DW_AT_bit_offset`、`DW_AT_bit_size` 四个属性来计算绝对位偏移。

### 修复方案

#### 修改 1: `src/lib/elf.rs` — 在创建 A2lEntry 时计算绝对 LSB 偏移

```rust
if member.is_bitfield() {
    let &(container_offset, container_size) = bitfield_groups.get(&member.offset);
    let container_addr = base_addr + container_offset as u64;
    let byte_in_container = member.offset.saturating_sub(container_offset);
    let storage_bits = member.type_size * 8;
    let raw_bo = member.bit_offset.unwrap_or(0);
    let raw_bs = member.bit_size.unwrap_or(0);

    // DWARF2: DW_AT_bit_offset 从存储单元 MSB 起算，转为绝对 LSB 偏移
    let actual_bit_offset = byte_in_container * 8
        + storage_bits.saturating_sub(raw_bo + raw_bs);

    store.add(
        A2lEntry::new(...)
            .with_bitfield(actual_bit_offset, raw_bs),  // 存绝对偏移
    );
}
```

#### 修改 2: `src/lib/types.rs` — get_effective_bit_offset 直接返回已转换的偏移

由于 A2lEntry.bit_offset 已是 LSB 绝对偏移，不再需要容器级别转换：

```rust
pub fn get_effective_bit_offset(&self, endianness: Endianness) -> Option<usize> {
    // bit_offset 已是绝对 LSB 偏移，直接使用
    Some(self.bit_offset?)
}
```

#### 修改 3 (可选): `src/lib/dwarf.rs` — 支持 DWARF4 的 DW_AT_data_bit_offset

DWARF4 引入的 `DW_AT_data_bit_offset` 已经是绝对 LSB 偏移，不需要转换。当前代码不支持此属性，建议添加以兼容未来工具链。

### 影响范围

所有包含 `DW_AT_data_member_location != 0` 的位域成员（即不在结构体首字节的位域），其位偏移均不正确。首字节位域（byte_offset=0）的偏移值虽然也不对（没加字节偏移），但如果容器是 8 位对齐且只有首字节位域，实际 BIT_MASK 计算结果可能碰巧正确。

### 相关文件

- `src/lib/elf.rs` — `expand_recursive` 函数（第 378-397 行）- bitfield A2lEntry 创建
- `src/lib/types.rs` — `A2lEntry::get_effective_bit_offset`（第 427-434 行）
- `src/lib/dwarf.rs` — `get_bitfield_info_static`（第 882-913 行）- DWARF 位域信息提取

---

## [FIXED] 超 8 字节位域组 BIT_MASK 无效，及无名 padding 位域生成假条目

**发现日期**: 2026-08-21
**修复日期**: 2026-08-21
**严重程度**: 高

### 问题描述

两个叠加问题（复现变量 `App_FunDegrad_Configs._6_.FunInhibitTable.*`，test.elf）：

1. 匿名结构体内 24 个 4 位字段构成 12 字节位域组，`compute_bitfield_groups`
   将整组当作一个"容器"：datatype 推断 12 字节失败回退 UBYTE（读宽 1 字节），
   掩码却按容器内绝对位偏移计算。后果：
   - `Fun08~Fun15` 掩码落在第 32~60 位，超出 UBYTE 读宽，INCA 取不到值
   - `Fun16~Fun22` 位偏移 ≥ 64，u64 移位回绕，掩码与 `Fun00~Fun06` 撞车
2. C 源码末尾的匿名 padding 位域（`unsigned char :4;`，DWARF 无 `DW_AT_name`）
   被起名 `"_"` 后继承父路径，生成顶着 `...FunInhibitTable` 名字的假条目，
   掩码同样是回绕错值（`App_FunDegrad_Cfg.h:27`，426 个 `...B` 条目同源）

### 根本原因

A2L 的 BIT_MASK 为 u64、datatype 最大 8 字节，"容器基址 + 容器内绝对位偏移"
的表示法只在容器 ≤ 8 字节时成立；NvM 的 7 字节容器也因 `infer` 对非标准尺寸
回退 UBYTE 而违反"datatype 由容器宽度推断"的原则。

### 修复方案（0.3.9）

`src/lib/elf.rs` `expand_bitfield` 引入读取窗口（`bitfield_read_window`）：

1. 容器宽度向上取整到 1/2/4/8 字节（≤4B 容器输出与原格式完全一致，
   5~8B 容器 datatype 修正为 A_UINT64，地址/掩码不变）
2. 位域超出 8 字节窗口时重锚定到能覆盖位域的最小对齐窗口，
   地址指向窗口起点，BIT_MASK 回到窗口内
3. 无名位域（padding）不再生成条目；无名复合成员仍正常展开
   （匿名 union/struct 依赖 `"_"` 占位名，见 2026-03-21 条目）

### 验证结果

- test.elf 全量导出 diff：252257 条中 244006 条完全一致；
  变化 8251 条全部归类为"datatype 拓宽 5770 / 重锚定 2481"，移除 748 条
  全部为无名 padding 位域（322 FunInhibitTable + 426 …B），新增 0
- test_cases.md 用例 1~6 按原标准零变化通过；用例 7 预期更新 datatype
- SF40_TC377_PRJ.elf 全量导出 0 警告 0 panic；EolTest_DcmService 位域（1B 容器）不变

---
