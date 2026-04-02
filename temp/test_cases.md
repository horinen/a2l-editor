# A2L Editor 回归测试用例

用于每次发版前验证 DWARF 解析和 A2L 生成功能。

## 测试文件
- ELF: `temp/test.elf`
- 数据包: `temp/test.a2ldata`

## 测试命令
```bash
# 每次测试前先清缓存
cargo run --bin a2l-cli -- clear
# 查询条目
cargo run --bin a2l-cli -- entries temp/test.elf "<搜索词>" -n 10
```

---

## 测试用例列表

### 1. volatile 修饰的数组类型

**搜索词**: `Dem_Cfg_StatusData.EventStatus`

**预期结果**:
```
Dem_Cfg_StatusData.EventStatus @ 0x700270BC 323B UBYTE
Dem_Cfg_StatusData.EventStatus._0_ @ 0x700270BC   1B UBYTE [0]
Dem_Cfg_StatusData.EventStatus._1_ @ 0x700270BD   1B UBYTE [1]
... (共 324 条：1 个数组 + 323 个元素)
```

**关键验证点**:
- 数组大小必须为 323B（不是 0B）
- 必须展开 323 个数组元素
- 每个元素大小为 1B，地址连续递增

**历史问题**:
- 2026-03: volatile 类型偏移量计算错误，导致 size=0
- 修复: `parse_volatile_type_with_offset` 使用 `get_type_offset_with_unit`

---

### 2. 位域成员共享容器的地址和类型

**搜索词**: `Cdd_L9388_FaultDebInfo._0_`

**预期结果**:
```
Cdd_L9388_FaultDebInfo._0_                  @ 0x7002734C   2B UWORD [0]
Cdd_L9388_FaultDebInfo._0_.FltDebValue      @ 0x7002734C   2B UWORD bits[0,13]
Cdd_L9388_FaultDebInfo._0_.CurrentDebStatus @ 0x7002734C   2B UWORD bits[14,15]
```

**源码定义**:
```c
typedef struct {
    uint16 FltDebValue       : 14;
    LibsFltDeb_DebStatusType CurrentDebStatus : 2;
} LibsFltDeb_DebHandleInfoType;
LibsFltDeb_DebHandleInfoType Cdd_L9388_FaultDebInfo[CDD_L9388_FLT_NUM];
```

**关键验证点**:
- `CurrentDebStatus` 地址必须与 `FltDebValue` 相同（`0x7002734C`），两者共享同一个 `uint16` 容器
- `CurrentDebStatus` size 必须为 `2B`（容器大小），不能是 `1B`（字段类型大小）
- `CurrentDebStatus` a2l_type 必须为 `UWORD`（由 2B 容器推断），不能是 `UBYTE`
- `FltDebValue` 的 bits 范围为 `[0,13]`（14 位），`CurrentDebStatus` 的 bits 范围为 `[14,15]`（2 位）
- 所有 227 个数组元素的位域成员都必须遵循相同规则（可用搜索词 `Cdd_L9388_FaultDebInfo` 验证，共 682 条）

**历史问题**:
- 2026-04: 位域成员使用 `member.offset` 和 `member.type_size` 而非容器信息
  - `CurrentDebStatus` 错误显示为 `@ 0x7002734D 1B UBYTE bits[0,1]`
  - 地址偏了 +1（用了字段自身的字节偏移而非容器起始）
  - size 为 1B（用了字段类型大小而非容器大小 2B）
  - type 为 UBYTE（由 1B 推断而非 2B → UWORD）
- 修复: 新增 `compute_bitfield_groups` 函数，将连续位域归为同一容器组，使用容器起始偏移和容器大小

---

### 3. DWARF2 位域绝对 LSB 偏移计算

**搜索词**: `EXCEPTIONS_CH0_5._0_.B`

**预期结果**:
```
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.Crc          @ 0x70027AAC   4B ULONG bits[0,4]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.LS_CLAMP_ON  @ 0x70027AAC   4B ULONG bits[5,5]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.LS_OVC       @ 0x70027AAC   4B ULONG bits[6,6]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.VGS_LS_FAULT @ 0x70027AAC   4B ULONG bits[7,7]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.VGS_HS_FAULT @ 0x70027AAC   4B ULONG bits[8,8]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.HS_SHORT     @ 0x70027AAC   4B ULONG bits[9,9]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.LVT          @ 0x70027AAC   4B ULONG bits[10,10]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD    @ 0x70027AAC   4B ULONG bits[11,11]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.GND_LOSS     @ 0x70027AAC   4B ULONG bits[12,12]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.TH_WARN      @ 0x70027AAC   4B ULONG bits[14,14]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.T_SD         @ 0x70027AAC   4B ULONG bits[15,15]
Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.Cmd          @ 0x70027AAC   4B ULONG bits[21,31]
```

**源码定义**:
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
        uint32              :1;   // byte 1, bit 13 (匿名，不输出)
        uint32 TH_WARN      :1;   // byte 1, bit 14
        uint32 T_SD         :1;   // byte 1, bit 15
        uint32              :5;   // byte 2 (匿名)
        uint32 Cmd          :11;  // byte 2-3, bits[21..31]
    } B;
} EXCEPTIONS_CH0_5[6];
```

**关键验证点**:
- `byte_offset != 0` 的位域成员必须正确计算绝对 LSB 偏移
- `Crc`(5bit): bits[0,4] — byte_offset=0, DWARF bit_offset=3 → `0*8+(8*4-3-5)=0` ✓... 实际存储单元为 uint32(4B)，但 DWARF2 DW_AT_byte_size 可能按更小单元报告
- `OPEN_LOAD`(1bit): bits[11,11] — byte_offset=1, DWARF bit_offset=4 → `1*8+(8-4-1)=11`
- `Cmd`(11bit): bits[21,31] — byte_offset=2, DWARF bit_offset=0 → `2*8+(32-0-11)=21`
- 所有位域成员共享同一个 `uint32` 容器，地址均为 `0x70027AAC`，size 均为 `4B ULONG`
- BIT_MASK 计算基于绝对 LSB 偏移：OPEN_LOAD 的 `bit_offset=11` → `BIT_MASK = 0x800`

**历史问题**:
- 2026-04: DWARF2 的 `DW_AT_bit_offset` 从存储单元 MSB 起算，代码未结合 `DW_AT_data_member_location` 转换为绝对 LSB 偏移
  - `OPEN_LOAD` 错误显示为 `bits[4,4]`（直接用了 DWARF 原始 bit_offset=4）
  - `Cmd` 错误显示为 `bits[0,10]`（未加 byte_offset*8 的偏移）
  - `A2lEntry::get_effective_bit_offset` 还用容器大小(32bit)做了一次错误的二次转换
- 修复:
  - `elf.rs`: 创建 A2lEntry 时计算绝对 LSB 偏移 `byte_in_container * 8 + (storage_bits - raw_bo - raw_bs)`
  - `types.rs`: `A2lEntry::get_effective_bit_offset` 和 `StructMember::get_effective_bit_offset` 直接返回已转换的偏移

---
