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
