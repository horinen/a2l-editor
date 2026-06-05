# DWARF 解析与 A2L 展开改进计划

## 背景

当前 DWARF 解析和 A2L 条目展开已经能覆盖主要场景，但近期几个回归问题暴露出一个共同弱点：类型解析阶段和条目展开阶段之间存在过多隐式假设。`TypeInfo` 在解析过程中会被多次克隆、拍平和补齐，数组、typedef、const、volatile、bitfield struct 组合后容易出现旧克隆、降级展开或旧数据包误用。

本计划的目标不是一次性推倒重写，而是按风险从低到高逐步收敛语义：先补测试和规则，再加数据包失效机制，最后整理类型解析架构。

## 总体原则

- 优先保护现有用户路径，不做大爆炸式重构。
- 先把展开规则写清楚并用测试锁住，再调整内部实现。
- 对外仍输出扁平化 `TypeInfo` 和 `A2lEntry`，避免一次性影响 CLI、Tauri、数据包序列化和 A2L 生成。
- 大 ELF 回归用于人工或可选验证，小型 synthetic 测试用于日常自动化。
- 旧数据包不能静默复用到新解析逻辑上。

## 优先级

1. 补齐 `elf.rs` 展开规则测试，并明确数组展开策略。
2. 增加数据包 parser version，避免旧 `.a2ldata` 被静默加载。
3. 抽出独立 `TypeResolver`，收束 DWARF 类型修正流程。
4. 在 Resolver 内部区分别名类型和真实类型。
5. 弱化或移除 `same_resolved_type()` 这类补丁式比较逻辑。

## 阶段 1：明确数组展开规则并补测试

### 目标

把当前隐含在 `expand_array`、`expand_multi_dim_array`、`expand_bitfield` 中的行为变成明确规则，并用单元测试覆盖。

### 需要明确的规则

- primitive/enum 数组：展开到最内层元素。
- nested primitive/enum 数组：保留所有维度索引，最内层元素按真实元素大小生成。
- bitfield struct/union 数组：为每个数组元素生成容器条目，并继续展开位域成员。
- normal struct/union 数组：是否生成元素容器需要明确。建议默认只展开 leaf members，不额外生成容器，除非 A2L 需求明确要求。
- 类型解析失败时的 fallback 必须显式命名，不应静默伪装成正常策略。

### 测试范围

- 一维 primitive 数组。
- 二维 enum 数组。
- const 二维 enum 数组。
- bitfield struct 数组。
- nested bitfield struct 数组。
- normal struct 数组。
- 类型缺失或元素类型未知的降级路径。

### 验收标准

- `elf.rs` 展开核心路径有 synthetic 单元测试。
- 每条展开规则能对应至少一个测试。
- 修改展开逻辑时，测试能定位是哪类规则变化。

## 阶段 2：数据包版本与失效机制

### 目标

避免解析逻辑已经修复，但 UI/CLI 仍加载旧 `.a2ldata` 导致用户看到旧结果。

### 方案

- 在数据包 meta 中增加 parser version。
- parser version 初期使用 `CARGO_PKG_VERSION`。
- 加载数据包时比较当前版本和包内版本。
- 版本不一致时拒绝加载或明确提示需要重建。
- 对旧包缺少 parser version 的情况，视为不兼容并提示重建。

### 验收标准

- 旧数据包不会被静默加载。
- CLI 和 Tauri 加载路径都能给出清晰错误。
- 生成新数据包后 meta 中包含 parser version。
- 数据包 schema 兼容迁移路径明确。

## 阶段 3：抽出 TypeResolver

### 目标

把现在分散在 `resolve_type_graph`、`resolve_array_element_types`、`resolve_alias_chains` 中的修正逻辑集中到一个解析阶段，减少多轮补丁式收敛。

### 建议边界

- `DwarfParser` 负责收集 DWARF 原始信息。
- `TypeResolver` 负责把原始类型图解析为稳定、扁平化的 `TypeInfo`。
- `ElfParser` 和 A2L 展开继续消费扁平化后的 `TypeInfo`。

### Resolver 需要处理的关系

- typedef、const、volatile 的 target。
- array 的 element target 和 dimensions。
- struct/union member 的 target。
- enum 的 variants。
- pointer target。
- 循环引用和最大深度保护。

### 验收标准

- 解析流程从多个互相依赖的修正函数，收敛为一个清晰的 Resolver 阶段。
- 现有 CLI 查询结果保持一致或按测试预期变化。
- `same_resolved_type()` 不再承担核心正确性职责。

### 当前进展

- 已抽出 `TypeResolver`，并将 array element、typedef/const/volatile alias、struct/union member 的类型修正收敛到 `resolve_type_by_offset()` 递归解析。
- 已移除旧的 `resolve_type_graph()` 多轮拍平流程，alias 链不再依赖固定 32 轮刷新。
- 已补充 synthetic 单测覆盖深 alias 链、member 经 alias 指向 array、alias cycle、array self-cycle、missing member type offset。
- `same_resolved_type()` 仍保留为避免等价类型重复刷新和旧克隆差异判断的保护，不再作为 alias 链解析的主要正确性来源。
- 对外仍输出扁平化 `TypeInfo`，CLI/Tauri/数据包格式未改变。

## 阶段 4：Resolver 内部区分别名和真实类型

### 目标

提升类型模型语义正确性，但不强迫所有消费者理解引用式类型。

### 方向

- Resolver 内部保留 typedef/const/volatile/array 的 target offset。
- Resolver 输出仍然是当前消费者可用的扁平化 `TypeInfo`。
- 别名名和真实类型名的保留规则需要明确。

### 验收标准

- const/typedef 嵌套 array 不依赖多轮克隆刷新。
- array 维度和元素类型由 offset 解析得到，不受 HashMap 遍历顺序影响。
- 对外 API 不需要大规模改动。

### 当前进展

- Resolver 内部已引入私有 `ResolvedType`，开始区分对外扁平 `flat TypeInfo`、alias 链背后的 `real_offset`，以及内部 alias offset 路径。
- 阶段 4 当前目标已基本完成：内部 resolver 已能同时携带 flat 输出、真实目标 offset 和 alias path，外部 API 与数据包格式保持不变。
- `resolve_type_by_offset()` 仍返回对当前消费者兼容的扁平类型，同时保留真实目标 offset 供后续 resolver 内部规则使用。
- 已补充 synthetic 单测验证 alias 链解析时外层 name/offset 保留在 flat 输出中，真实目标 offset 保留在 `real_offset` 中。
- `real_offset` 已用于 array element 解析中的 alias 间接自循环检测，顶层和递归 array 解析都会避免把数组自身或未解析 0-size alias 写回 `pointer_target`。

## 阶段 5：集成回归测试（已取消）

> 决定：暂不推进阶段 5，不再把大 ELF 或小 ELF fixture 沉淀为自动化集成回归。真实 ELF 场景仅保留为本地手工抽样验证，不作为后续计划或提交内容。

### 原计划内容

#### 目标

把人工验证过的大 ELF 场景沉淀成可重复回归。

### 建议分层

- 快速单元测试：构造 `TypeInfo` 或 Resolver 输入，不依赖文件。
- 小型 ELF 夹具：覆盖真实 DWARF DIE 解析，但文件要小。
- 大 ELF 手工回归：保留本地脚本或文档，不进入默认 CI。

### 关键场景

- `Cdd_TLE918X_FaultDebInfo`：nested bitfield struct array。
- `Cdd_WheelSpeed_Fault`：二维 bitfield struct array。
- `Cdd_L9388_WssFaultIdConfigs`：const 二维 enum array。
- `Dem_Cfg_StatusData.EventStatus`：volatile array。
- `Cdd_L9388_FaultDebInfo`：bitfield container sharing。
- `Cdd_L9388_SFR_RX.EXCEPTIONS_CH0_5`：跨字节 bitfield。

### 验收标准

- 默认 `cargo test` 不依赖大型私有 ELF。
- 大 ELF 回归有清晰命令和预期输出。
- 回归用例不放在 ignored `temp/` 路径中。

## 风险与注意事项

- 改动类型解析容易导致条目数量大幅变化，需要解释哪些变化是修复、哪些是行为变化。
- bitfield 容器条目生成规则会影响 A2L 输出数量，必须由测试固定。
- 数据包版本机制可能让用户首次升级后必须重建数据包，需要 UI/CLI 文案配合。
- 如果后续引入小 ELF 夹具，需要确认是否可提交到仓库，避免泄露业务内容或引入大文件。

## 建议执行顺序

1. 先为 `elf.rs` 添加展开规则测试。
2. 把数组展开规则写到设计文档或 `elf.rs` 相关函数注释中。
3. 加 parser version，并覆盖 CLI/Tauri 加载路径。
4. 梳理 `DwarfParser` 当前原始信息收集能力，设计 `TypeResolver` 输入输出。
5. 小步替换现有 resolve 流程，每一步都用现有回归场景验证。

## 完成定义

- 核心展开策略有文档、有自动测试。
- 旧数据包不会静默污染验证结果。
- DWARF 类型解析流程职责清晰，收集和解析分离。
- 新增复杂变量类型时，不需要继续叠加补丁式 resolve 函数。
