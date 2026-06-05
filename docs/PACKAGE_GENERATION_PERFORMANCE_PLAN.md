# 数据包生成性能优化计划

## 背景

当前大 ELF 首次生成 `.a2ldata` 的耗时约 85 秒，生成后从数据包加载约 700~800ms。UI 和已有数据包加载速度可接受，因此优化重点不放在 UI 加载、数据包格式迁移或懒加载，而是聚焦首次深度解析和数据包生成链路。

## 目标

- 缩短 `a2l-cli entries <elf>` 首次生成数据包的耗时。
- 保持 `.a2ldata` 格式和 UI 加载路径稳定。
- 优先用分段耗时定位瓶颈，再做小范围优化。
- 不提交大 ELF、生成数据包或 `temp/` 文件。

## 当前链路

首次生成数据包大致包含：

1. 读取 ELF 并遍历 DWARF DIE。
2. 构建 `type_cache`、`type_refs`、`array_elem_offsets`、变量列表。
3. `TypeResolver` 解析 array、alias、member、bitfield offset。
4. 从 DWARF 变量生成 `Variable`。
5. 展开全部 A2L entries。
6. 序列化并写入 `.a2ldata`。

## 第一阶段：分段耗时统计

先在 CLI/Tauri 共享的数据包生成路径附近增加分段计时，输出或记录：

- ELF/DWARF parse 耗时。
- TypeResolver 耗时。
- 变量提取耗时。
- A2L entry 展开耗时。
- 数据包序列化和写盘耗时。
- 总条目数、类型数、变量数。

验收标准：

- 不改变数据包格式。
- 不影响已有数据包加载。
- `cargo test` 通过。
- 使用 `temp/test.elf` 能看到清晰分段耗时，并清理生成的 `.a2ldata` 和 `.lock`。

### 第一阶段实测记录

测量命令：

```bash
cargo run --bin a2l-cli -- entries temp/test.elf "Dem_Cfg_StatusData.EventStatus" -n 5 --a2l
```

首次生成输出已在 CLI 中打印 `=== 首次生成耗时分布 ===`，仅在缺少数据包、实际执行深度解析并保存 `.a2ldata` 时出现；已有数据包加载路径不打印该诊断信息。

2026-06-05 本地大 ELF 测量结果：

- ELF/object parse: 1 ms
- DWARF parse 总计: 75.781 s
  - DWARF DIE 遍历: 30.428 s
  - TypeResolver: 45.322 s
- 变量提取: 56 ms
- type_cache clone: 5.097 s
- A2L entry 展开: 778 ms
- 数据包序列化/写盘: 1.051 s
- 解析小计: 81.714 s
- 生成总计: 82.765 s
- 计数: 类型 2500273, DWARF 变量 15233, 有效变量 15213, A2L 条目 253005

验收查询结果：总条目数 253005，`Dem_Cfg_StatusData.EventStatus` 匹配 323 条。

初步结论：瓶颈主要在 `TypeResolver`，其次是 DWARF DIE 遍历；A2L entry 展开和数据包写盘占比很低。下一步若继续优化，应优先验证 resolver offset memo cache 的收益和正确性风险，同时留意 5 秒级 `type_cache` clone 成本。

### 第二阶段第一次优化记录

尝试过在 `TypeResolver::resolve_type_by_offset()` 内增加按 offset 缓存 `ResolvedType`，并保持每个 resolver 阶段独立 memo、循环检测优先于缓存。真实 ELF 验证中条目数保持正确，但 `TypeResolver` 仍约 45.242 s，对比基线 45.322 s 基本无收益，因此未保留该优化，避免增加 resolver 语义复杂度。

保留的低风险优化：`ElfParser::parse_deep()` 不再为了内部 `type_cache` 字段额外克隆 2500273 个类型，A2L entry 展开直接借用 `DwarfParser` 的 `type_cache`。该字段原本仅作为可选内部缓存保存，不参与 CLI/Tauri 数据包生成结果。

优化后测量结果：

- DWARF parse 总计: 75.941 s
  - DWARF DIE 遍历: 30.596 s
  - TypeResolver: 45.313 s
- 变量提取: 58 ms
- type_cache clone: 0 ms
- A2L entry 展开: 786 ms
- 数据包序列化/写盘: 1.041 s
- 解析小计: 76.787 s
- 生成总计: 77.829 s

验收查询结果保持不变：总条目数 253005，`Dem_Cfg_StatusData.EventStatus` 匹配 323 条。

下一步瓶颈仍然是 `TypeResolver` 和 DWARF DIE 遍历。resolver 优化需要更细的调用计数或按子阶段耗时统计，不能直接假设简单 offset memo 有收益。

### TypeResolver 子阶段测量记录

新增 `TypeResolver` 子阶段耗时输出后，真实 ELF 测量结果：

- TypeResolver: 45.098 s
  - array element types: 402 ms
  - alias chains: 8.168 s
  - member types: 35.729 s
  - bitfield normalize: 798 ms

结论：`member types` 是 resolver 内部主要瓶颈，其次是 `alias chains`。尝试过在 member 阶段跳过无变化写回，真实 ELF 上 `member types` 仍约 36.248 s，无明显收益，因此未保留。下一步需要针对 `resolve_member_types()` 的递归解析调用次数、重复 type_offset 分布、`TypeInfo` clone 成本继续加计数或做更聚焦优化。

### Member type 局部缓存优化记录

进一步统计 `resolve_member_types()` 后发现：

- member type 解析次数: 19351452
- unique type offsets: 670305
- member updates: 19351452

重复度很高，因此在 member 阶段内部增加局部缓存：`type_offset -> Option<(type_name, type_size)>`。缓存只用于 member type 名称和大小更新，不跨 resolver 阶段复用，不改变 array/alias 阶段的解析语义。

优化后真实 ELF 测量结果：

- TypeResolver: 27.158 s
  - array element types: 389 ms
  - alias chains: 8.109 s
  - member types: 17.837 s
  - bitfield normalize: 822 ms
- DWARF parse 总计: 58.201 s
- 解析小计: 59.096 s
- 生成总计: 60.119 s

验收查询结果保持不变：总条目数 253005，`Dem_Cfg_StatusData.EventStatus` 匹配 323 条。

下一步热点变为 DWARF DIE 遍历约 31 s、member types 约 18 s、alias chains 约 8 s。

### Alias chain 诊断记录

新增 alias 阶段计数后，真实 ELF 测量结果：

- aliases: 1481800
- resolved: 1481800
- zero-size: 47953
- same-resolved: 0
- updates: 1433847
- alias chains 耗时: 8.161 s

尝试过移除 alias 阶段的 `same_resolved_type()` 深比较，改为总是写回 resolved flat 类型；真实 ELF 上 alias chains 仍约 8.221 s，无明显收益，因此未保留。当前判断：alias 阶段耗时主要来自 148 万次 alias 解析和写回，而不是 same-resolved 深比较。

### DWARF DIE 遍历诊断记录

新增 DIE/tag 计数后，真实 ELF 测量结果：

- DWARF DIE 遍历: 30.410 s
- units: 35741
- DIEs: 14327482
- members: 9611519
- variables: 45508
- composites saved: 906082
- other tags: 1225748

结论：DIE 遍历阶段也主要被大量 `DW_TAG_member` 驱动。下一步若继续优化 DWARF parse，应优先聚焦 member 解析路径（名称读取、member location/type offset/bitfield attr 读取、成员暂存分配），而不是泛泛优化所有 tag。

### DWARF name 读取诊断记录

新增 name 读取计数后，真实 ELF 测量结果：

- name attrs: 12180356
- debug_str refs: 0
- debug_str cache hits: 0
- debug_str cache misses: 0

尝试过为 `.debug_str` offset 增加字符串缓存，但该 ELF 的名称全部以内联 string 形式出现，`DebugStrRef` 为 0，因此缓存无收益并已撤回。下一步若继续优化 name 读取，应聚焦 `String::from_utf8_lossy()`/字符串分配本身，或评估 member 名称是否能延迟/共享，而不是缓存 `.debug_str`。

进一步尝试过为 inline name 内容增加 `HashMap<Vec<u8>, String>` 缓存。真实 ELF 命中率很高（12100410 hits, 79946 misses），但 DIE 遍历从约 30.6 s 上升到约 35.6 s，说明按内容 HashMap 查找和 key 构造成本超过避免字符串转换的收益，因此已撤回。

### struct_map 存储优化记录

`struct_map` 原本保存 `HashMap<String, TypeInfo>`，在保存 named struct/union 时会额外 clone 一份完整 `TypeInfo`，但该 map 主要用于 CLI inspect 诊断，不参与数据包生成。已改为 `HashMap<String, u64>` 保存 type offset，查询时再从 `type_cache` 返回 `&TypeInfo`。

真实 ELF 上首次生成时间没有明显下降（生成总计约 60.223 s），但避免了 named struct/union 在 `struct_map` 中重复保存大对象，降低内存占用和 clone 风险，且对外 inspect API 返回值保持为 `&TypeInfo`。

### DW_TAG_member 属性读取合并优化记录

`parse_member()` 原先对每个 member 分别调用多个 helper，重复查询 `DW_AT_name`、`DW_AT_data_member_location`、`DW_AT_byte_size`、`DW_AT_type`、`DW_AT_bit_size`、`DW_AT_bit_offset`、`DW_AT_data_bit_offset`。真实 ELF 有 9611519 个 `DW_TAG_member`，重复 attr 查找成本很高。

已改为 `parse_member_attrs()` 对单个 member DIE 做一次 `entry.attrs()` 遍历，同时收集 name、offset、size、type offset 和 bitfield 信息。

优化后真实 ELF 测量结果：

- DWARF DIE 遍历: 17.607 s
- DWARF parse 总计: 44.889 s
- 解析小计: 45.793 s
- 生成总计: 46.852 s

验收查询结果保持不变：总条目数 253005，`Dem_Cfg_StatusData.EventStatus` 匹配 323 条。

### Member type 写回表去除优化记录

`resolve_member_types()` 原先在遍历 struct/union member 时，一边解析 type offset，一边构造 `Vec<(offset, Vec<(idx, type_name, type_size)>)>` 中间写回表。真实 ELF 中 member 更新数为 19351452，这会额外产生大量临时 Vec、tuple 和字符串 clone。

已改为两阶段处理：先扫描全部成员收集唯一 `type_offset` 并解析成局部缓存，再第二次遍历 struct/union 就地写回成员 `type_name` 和 `type_size`。该缓存仍只限 member 阶段内部，不跨 resolver 阶段复用，不改变 `.a2ldata` 格式或 resolver 对外语义。

优化后真实 ELF 测量结果：

- DWARF parse 总计: 40.258 s
  - DWARF DIE 遍历: 17.341 s
  - TypeResolver: 22.886 s
- TypeResolver 子阶段：
  - array element types: 386 ms
  - alias chains: 8.044 s
  - member types: 13.684 s
  - bitfield normalize: 771 ms
- 解析小计: 41.110 s
- 生成总计: 42.195 s

验收查询结果保持不变：总条目数 253005，`Dem_Cfg_StatusData.EventStatus` 匹配 323 条。

### Resolver recursion stack Vec optimization record

`resolve_type_by_offset()` previously created a `HashSet<u64>` for cycle detection on every resolution. Alias resolution runs 1481800 resolutions, and member resolution resolves 670305 unique `type_offset` values, so these small HashSet allocations and hash lookups are measurable.

Changed the recursion guard to a `Vec<u64>` that represents the current call stack: check `contains()`, then `push()`, and `pop()` on return. The resolver only needs to know whether the current stack already contains an offset, and the real alias/array depth is much smaller than the number of calls.

Optimized real ELF result:

- DWARF parse total: 37.323 s
  - DWARF DIE traversal: 17.188 s
  - TypeResolver: 20.104 s
- TypeResolver detail:
  - array element types: 337 ms
  - alias chains: 6.500 s
  - member types: 12.499 s
  - bitfield normalize: 767 ms
- parse subtotal: 38.166 s
- generation total: 39.207 s

Validation result unchanged: total entries 253005, `Dem_Cfg_StatusData.EventStatus` matches 323 entries.

### Member type direct cache read optimization record

After `resolve_array_element_types()` and `resolve_alias_chains()` complete, member resolution only needs each member target type name and size. The previous member cache still called `resolve_type_by_offset()` for every unique member `type_offset`, cloning resolved `TypeInfo` values even though the canonical `type_cache` entry had already been updated by earlier resolver stages.

Changed member cache construction to read `(name, size)` directly from the already-resolved `type_cache`. This keeps the member stage local and does not change array/alias resolution order, package format, or parser version behavior.

Optimized real ELF result:

- DWARF parse total: 35.022 s
  - DWARF DIE traversal: 17.204 s
  - TypeResolver: 17.787 s
- TypeResolver detail:
  - array element types: 337 ms
  - alias chains: 6.526 s
  - member types: 10.145 s
  - bitfield normalize: 777 ms
- parse subtotal: 35.866 s
- generation total: 36.866 s

Validation result unchanged: total entries 253005, `Dem_Cfg_StatusData.EventStatus` matches 323 entries.

### Member temporary type name allocation optimization record

`parse_member()` previously initialized every `StructMember` with the owned string `"unknown"` as a temporary `type_name`. On the real ELF there are 9611519 `DW_TAG_member` entries, and members with a valid `type_offset` later have `type_name` overwritten by resolver output.

Changed member parsing to use an empty string placeholder when `type_offset > 0`, while preserving `"unknown"` for members without a type offset. This avoids allocating and then discarding millions of identical temporary strings without changing the resolved member names or package output.

Optimized real ELF result:

- DWARF parse total: 32.764 s
  - DWARF DIE traversal: 16.585 s
  - TypeResolver: 16.148 s
- TypeResolver detail:
  - array element types: 323 ms
  - alias chains: 6.105 s
  - member types: 8.944 s
  - bitfield normalize: 775 ms
- parse subtotal: 33.606 s
- generation total: 34.651 s

Validation result unchanged: total entries 253005, `Dem_Cfg_StatusData.EventStatus` matches 323 entries.

### 当前最终基线

截至本轮优化后的最终验证结果：

- 深度解析完成: 253005 条目, 耗时 38762 ms
- DWARF parse 总计: 32.764 s
  - DWARF DIE 遍历: 16.585 s
  - TypeResolver: 16.148 s
- 变量提取: 65 ms
- A2L entry 展开: 774 ms
- 数据包序列化/写盘: 1.045 s
- 解析小计: 33.606 s
- 生成总计: 34.651 s

验收查询结果保持不变：总条目数 253005，`Dem_Cfg_StatusData.EventStatus` 匹配 323 条。

继续优化空间判断：剩余热点主要是 `TypeResolver` 的 member types 约 8.9 s、alias chains 约 6.1 s，以及 DWARF DIE 遍历约 16.6 s。进一步压缩需要更深层的类型模型/字符串所有权/alias 链解析语义调整，风险明显高于本轮保留的局部优化；本轮停止继续修改，保留已验证收益明确的改动。

## 第二阶段：按瓶颈优化

### Resolver 重复解析

如果 `TypeResolver` 占比明显：

- 给 resolver 内部增加 offset memo cache。
- 缓存 `ResolvedType`，避免同一 offset 在 array、alias、member 路径中重复递归解析。
- 保留 cycle detection 的局部 `visiting` 语义，避免缓存未完成的循环状态。

风险：

- 需要确保 alias path、real offset、flat 输出不被错误复用。
- 必须保留现有 synthetic tests，并补充缓存命中下的 alias/array cycle 测试。

### DWARF DIE 遍历

如果 DWARF parse 占比最高：

- 统计各 DIE tag 数量和处理耗时。
- 只优化明显热点，例如重复字符串解析、无关 DIE 处理、过度 clone。
- 不改变当前类型收集覆盖范围，避免真实回归丢字段。

### A2L entry 展开

如果展开 253005 条 entries 占比最高：

- 优先优化内部遍历和分配，暂不改成懒展开。
- 评估 `A2lEntry` 构造中的字符串分配和重复 prefix 生成。
- 保持 UI/CLI 能继续从数据包快速加载完整 entries。

### 数据包写盘

如果序列化/写盘占比最高：

- 评估序列化耗时和文件大小。
- 暂不改数据包格式，除非有明确收益和兼容策略。
- 保持 parser_version 失效机制不变。

## 不做事项

- 不优化已有 `.a2ldata` 的 UI 加载路径。
- 不引入小 ELF fixture 或阶段 5 集成回归。
- 不提交 `temp/` 下生成物。
- 不为了性能改变 A2L entry 数量或命名规则。

## 推荐执行顺序

1. 增加分段耗时统计。
2. 用 `temp/test.elf` 跑一次生成，记录各阶段耗时。
3. 根据占比选择一个瓶颈做小步优化。
4. 每个优化后跑 `cargo test` 和真实 ELF 抽样。
5. 清理生成的 `.a2ldata` 和 `.lock`。
