# A2L Editor 回归测试用例

验证 DWARF 解析和 A2L 生成功能是否正确。

## 测试文件
- ELF: `temp/test.elf`
- 数据包: `temp/test.a2ldata`

## 前置步骤
```bash
cargo run --bin a2l-cli -- clear
```

---

## 1. volatile 修饰的数组类型

验证 volatile 类型数组能正确计算偏移量并展开所有元素。

**步骤**:
```bash
cargo run --bin a2l-cli -- entries temp/test.elf "Dem_Cfg_StatusData.EventStatus" -n 5 --a2l
```

**预期**（共 324 条，此处列前 2 条）:
```
/begin MEASUREMENT Dem_Cfg_StatusData.EventStatus ""
  UBYTE NO_COMPU_METHOD 0 0 0 255
  ECU_ADDRESS 0x700270BC
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%3.0"
  SYMBOL_LINK "Dem_Cfg_StatusData.EventStatus" 0
/end MEASUREMENT
/begin MEASUREMENT Dem_Cfg_StatusData.EventStatus._0_ ""
  UBYTE NO_COMPU_METHOD 0 0 0 255
  ECU_ADDRESS 0x700270BC
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%3.0"
  SYMBOL_LINK "Dem_Cfg_StatusData.EventStatus._0_" 0
/end MEASUREMENT
```

**注意**: 数组大小必须为 323B（不是 0B），323 个元素地址连续递增。

---

## 2. 位域成员共享容器

验证同一容器内多个位域成员共享相同的地址、大小、类型，且 BIT_MASK 正确。

**步骤**:
```bash
cargo run --bin a2l-cli -- entries temp/test.elf "Cdd_L9388_FaultDebInfo._0_." -n 5 --a2l
```

**预期**:
```
/begin MEASUREMENT Cdd_L9388_FaultDebInfo._0_.CurrentDebStatus ""
  UWORD NO_COMPU_METHOD 0 0 0 3
  BIT_MASK 0xC000
  ECU_ADDRESS 0x7002734C
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%5.0"
  SYMBOL_LINK "Cdd_L9388_FaultDebInfo._0_.CurrentDebStatus" 0
/end MEASUREMENT
/begin MEASUREMENT Cdd_L9388_FaultDebInfo._0_.FltDebValue ""
  UWORD NO_COMPU_METHOD 0 0 0 16383
  BIT_MASK 0x3FFF
  ECU_ADDRESS 0x7002734C
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%5.0"
  SYMBOL_LINK "Cdd_L9388_FaultDebInfo._0_.FltDebValue" 0
/end MEASUREMENT
```

**注意**:
- 两个位域成员 ECU_ADDRESS 必须相同（共享 uint16 容器）
- 类型必须为 UWORD（2B 容器），不能是 UBYTE
- BIT_MASK: `FltDebValue`(14bit) → `0x3FFF`，`CurrentDebStatus`(2bit) → `0xC000`

---

## 3. DWARF2 跨字节位域的绝对 LSB 偏移

验证跨字节位域（byte_offset != 0）能正确计算绝对 LSB 偏移和 BIT_MASK。

**步骤**:
```bash
cargo run --bin a2l-cli -- entries temp/test.elf "EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD" -n 1 --a2l
cargo run --bin a2l-cli -- entries temp/test.elf "EXCEPTIONS_CH0_5._0_.B.Cmd" -n 1 --a2l
```

**预期**:
```
/begin MEASUREMENT Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD ""
  ULONG NO_COMPU_METHOD 0 0 0 1
  BIT_MASK 0x800
  ECU_ADDRESS 0x70027AAC
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%10.0"
  SYMBOL_LINK "Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.OPEN_LOAD" 0
/end MEASUREMENT
```
```
/begin MEASUREMENT Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.Cmd ""
  ULONG NO_COMPU_METHOD 0 0 0 2047
  BIT_MASK 0xFFE00000
  ECU_ADDRESS 0x70027AAC
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%10.0"
  SYMBOL_LINK "Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5._0_.B.Cmd" 0
/end MEASUREMENT
```

**注意**:
- `OPEN_LOAD`: byte_offset=1, bit_offset=11 → BIT_MASK 必须为 `0x800`
- `Cmd`: byte_offset=2, bit_offset=21 → BIT_MASK 必须为 `0xFFE00000`
- 所有位域成员共享同一 ECU_ADDRESS `0x70027AAC`
