# COMPU_METHOD 编辑器设计方案

## 目标

为 A2L Editor 增加 ASAP2 Studio 风格的转换关系（COMPU_METHOD）编辑功能，包括：
1. 转换关系列表展示
2. 完善的 COMPU_METHOD 编辑器（新建/编辑/删除）
3. 变量关联转换关系（选择已有或新建并关联）
4. 转换预览（原始值 ↔ 物理值）

## 实现范围（第一期）

支持的 COMPU_METHOD 类型：
- **LINEAR** — 线性转换 `physical = f * raw + offset`（已有，增强）
- **TAB_VERB** — 文字-数值映射表（枚举值到文字描述）
- **TAB_INTP** — 线性插值表（分段线性插值）
- **IDENTICAL** — 无转换（原始值=物理值）

未在本期实现（留待后续）：RAT_FUNC、FORM、COMPU_VTAB、SCALE_LINEAR 等。

---

## 一、后端改动（Rust）

### 1.1 数据结构扩展

在 `src/lib/a2l.rs` 中：

**重构 `CompuMethod` 结构**，从只支持 LINEAR 扩展为支持多种类型。将当前的扁平结构（name/f/offset/unit）改为带类型标签的枚举式结构：

- `name: String` — COMPU_METHOD 名称
- `conversion_type: CompuMethodType` — 枚举：LINEAR / TAB_VERB / TAB_INTP / IDENTICAL
- `unit: String` — 单位
- `description: String` — LongIdentifier 描述

**每种类型的数据**：
- LINEAR：斜率 f 和偏移 offset（保持与当前兼容）
- TAB_VERB：值对列表 `Vec<(i32, String)>`（数值→文字描述）+ 默认值
- TAB_INTP：值对列表 `Vec<(f64, f64)>`（原始值→物理值，线性插值）
- IDENTICAL：无额外参数

### 1.2 解析器扩展（`A2lParser`）

扩展现有 `parse_compu_methods` 函数，使其能解析：

- `/begin COMPU_METHOD ... LINEAR ... COEFFS f offset 0 0 0 0 /end`（已有）
- `/begin COMPU_METHOD ... IDENTICAL ... /end`（新增）
- `/begin COMPU_METHOD ... TAB_VERB ... DEFAULT_VALUE ... COMPU_VTAB ... /end`（新增）
- `/begin COMPU_METHOD ... TAB_INTP ... COMPU_TAB ... /end`（新增）

解析逻辑：读取 conversion_type 字段后，按类型解析对应的数值表/系数。

### 1.3 生成器扩展（`A2lGenerator`）

新增方法：

- `generate_compu_method_block_generic(method)` — 根据 CompuMethod 类型生成对应的 A2L 文本块
  - LINEAR 沿用当前 `COEFFS` 格式
  - TAB_VERB 生成 `COMPU_VTAB` 格式
  - TAB_INTP 生成 `COMPU_TAB` 格式（`f` 为关键字，后跟值对数 + 各对值）
  - IDENTICAL 生成最简格式

- `save_compu_method(content, method)` — 创建或更新单个 COMPU_METHOD 块
  - 如果 name 已存在：替换整个 `/begin COMPU_METHOD ... /end COMPU_METHOD` 块
  - 如果 name 不存在：在合适位置插入新块

- `delete_compu_method(content, name)` — 删除指定 COMPU_METHOD 块
  - 返回是否有变量仍在引用它（前端用于确认提示）

- `preview_compu_method(method, raw_values)` — 给定原始值数组，返回物理值数组
  - LINEAR：`f * raw + offset`
  - TAB_INTP：在值对间线性插值，超出范围用端点值
  - TAB_VERB：返回对应文字（前端表格展示）
  - IDENTICAL：原样返回

### 1.4 新增 Tauri 命令

在 `src-tauri/src/commands.rs` 中新增：

| 命令 | 功能 |
|------|------|
| `list_compu_methods` | 列出当前 A2L 文件中所有 COMPU_METHOD，返回每个的名称、类型、摘要、关联变量数 |
| `get_compu_method` | 获取单个 COMPU_METHOD 完整详情 |
| `save_compu_method` | 新建或修改 COMPU_METHOD（写入 A2L 文件） |
| `delete_compu_method` | 删除 COMPU_METHOD（检查引用） |
| `preview_compu_method` | 转换预览，输入原始值范围/列表，返回物理值 |

在 `src-tauri/src/main.rs` 的 `invoke_handler` 中注册这 5 个新命令。

### 1.5 向后兼容

现有的 `apply_changes` 流程（通过 f/offset 自动管理 COMPU_METHOD）保持不变。用户通过变量编辑器修改 f/offset 仍会走当前逻辑。新的 COMPU_METHOD 编辑器是补充能力。

---

## 二、前端改动（Svelte）

### 2.1 类型定义（`src-ui/src/lib/types.ts`）

新增类型：

- `CompuMethodType` — 联合类型：'LINEAR' | 'TAB_VERB' | 'TAB_INTP' | 'IDENTICAL'
- `TabVerbPair` — `{ raw: number; verbal: string }`
- `TabIntpPair` — `{ raw: number; physical: number }`
- `CompuMethodDetail` — 完整的 COMPU_METHOD 数据，含 name、type、unit、description、以及按类型不同的数据字段
- `CompuMethodSummary` — 列表项：name、type、summary（公式/表项数摘要）、refCount（引用变量数）
- `PreviewResult` — 预览结果，原始值和物理值/文字的对应数组

### 2.2 命令封装（`src-ui/src/lib/commands.ts`）

新增 5 个函数对应后端命令：`listCompuMethods`、`getCompuMethod`、`saveCompuMethod`、`deleteCompuMethod`、`previewCompuMethod`。

### 2.3 新组件：CompuMethodPanel.svelte

独立的 COMPU_METHOD 管理面板，布局为左右分栏：

**左侧（列表区）**：
- 工具栏：搜索框 + "新建" 按钮 + 刷新按钮
- 列表：每行显示名称、类型标签（彩色）、摘要（公式或表项数）、引用数
- 点击选中后右侧显示详情
- 右键菜单：编辑、删除、复制名称

**右侧（编辑区）**，根据类型动态切换表单：

- **通用字段**（所有类型）：名称、单位、描述
- **LINEAR**：F（斜率）、OFFSET 输入框 + 实时预览
- **TAB_VERB**：可编辑表格（原始值、文字描述两列）+ 默认值输入 + 添加/删除行按钮
- **TAB_INTP**：可编辑表格（原始值、物理值两列）+ 添加/删除行按钮
- **IDENTICAL**：无额外字段，提示文字

**底部**：保存按钮 + 取消按钮

### 2.4 新组件：CompuMethodPreview.svelte

转换预览面板：
- 输入区：起始值、结束值、步长（或手动输入逗号分隔的值列表）
- 预览按钮
- 结果表格：原始值 → 物理值（TAB_VERB 类型显示文字）
- LINEAR/TAB_INTP 类型可绘制简单的数值对应示意

### 2.5 A2lEditor.svelte 增强

在现有的"转化系数 (COMPU_METHOD)"分隔线下：

- 新增一个"选择转换关系"下拉框，列出现有 COMPU_METHOD 名称
- 选择后自动填充 f/offset/unit（如果是 LINEAR 类型）
- 新增"管理转换关系"按钮，打开 CompuMethodPanel 弹窗/侧边栏
- 如果变量引用的 COMPU_METHOD 是非 LINEAR 类型，只显示名称和类型标签，不显示 f/offset 输入框

### 2.6 入口集成

在 `+page.svelte` 中集成 CompuMethodPanel：

方案：作为可切换的覆盖面板（类似 ExportDialog 的全屏弹窗），通过 Header 工具栏的"转换关系"按钮打开。这样不破坏现有布局，用户需要时才打开。

也可以做成 A2lPanel 底部的第三个可折叠区域（与 A2lEditor 并列的 Tab）。第一期采用弹窗方式，简单直接。

---

## 三、数据流

### 3.1 加载流程

1. 用户打开 A2L 文件 → `load_a2l` 命令解析变量（已有）
2. 用户点击"转换关系管理" → `list_compu_methods` 获取所有 COMPU_METHOD 列表
3. 列表展示，用户点选某个 → `get_compu_method` 获取详情

### 3.2 编辑保存流程

1. 用户在编辑区修改参数 → 点击保存
2. `save_compu_method` 写入 A2L 文件
3. 如果是修改现有 COMPU_METHOD，检查哪些变量引用了它，刷新前端缓存
4. 返回成功后刷新列表

### 3.3 变量关联流程

1. 用户在 A2lEditor 中选择变量
2. 从"选择转换关系"下拉框选一个 COMPU_METHOD
3. 前端构造 `A2lVariableEdit`，只设置 `compu_method` 字段（直接指定名称，不通过 f/offset）
4. 调用 `save_a2l_changes` → 后端 `modify_variable` 更新引用

### 3.4 预览流程

1. 用户输入原始值范围
2. 前端调用 `preview_compu_method`
3. 后端按类型计算并返回结果
4. 前端展示对照表

---

## 四、实现步骤

### 步骤 1：后端数据结构重构

- 在 `src/lib/a2l.rs` 中扩展 `CompuMethod` 结构
- 添加 `CompuMethodType` 枚举和相关子数据结构
- 更新 `parse_compu_method_block` 支持所有类型
- 更新 `generate_compu_method_block_generic`
- 确保现有 LINEAR 逻辑不受影响

### 步骤 2：后端 CRUD 方法

- 实现 `save_compu_method`（创建/更新单个块）
- 实现 `delete_compu_method`（删除单个块 + 引用检查）
- 实现 `preview_compu_method`（转换预览计算）
- 实现 `count_compu_method_refs`（统计变量引用数）

### 步骤 3：Tauri 命令层

- 在 `commands.rs` 中添加 5 个新命令
- 在 `main.rs` 中注册
- 在 `mod.rs` 中导出新类型

### 步骤 4：前端类型和命令封装

- 在 `types.ts` 中添加新类型定义
- 在 `commands.ts` 中添加命令封装函数

### 步骤 5：CompuMethodPreview 组件

- 实现预览面板
- 输入原始值范围，显示物理值对照表

### 步骤 6：CompuMethodPanel 组件

- 实现列表区（搜索、新建、选中）
- 实现编辑区（按类型切换表单）
- 集成预览面板

### 步骤 7：A2lEditor 增强

- 添加"选择转换关系"下拉框
- 添加"管理转换关系"入口按钮

### 步骤 8：入口集成

- 在 Header 或 A2lPanel 中添加"转换关系"按钮
- 在 `+page.svelte` 中挂载 CompuMethodPanel 弹窗

### 步骤 9：验证

- `cargo build && cargo test` 确保 Rust 编译和测试通过
- `cd src-ui && npm run check` 确保 Svelte 类型检查通过

---

## 五、A2L 格式参考

### LINEAR 类型

```
/begin COMPU_METHOD
  CM_F1_O0 "y = 1 * x + 0"
  LINEAR "%10.4" "" ""
  COEFFS 1 0 0.0 0.0 0.0 0.0
/end COMPU_METHOD
```

### IDENTICAL 类型

```
/begin COMPU_METHOD
  CM_IDENT "no conversion"
  IDENTICAL "" "" ""
/end COMPU_METHOD
```

### TAB_VERB 类型

```
/begin COMPU_METHOD
  CM_VERB_1 "verbal table"
  TAB_VERB "" "" ""
  DEFAULT_VALUE "Unknown"
  COMPU_VTAB 0 "Off" 1 "On" 2 "Error"
/end COMPU_METHOD
```

### TAB_INTP 类型

```
/begin COMPU_METHOD
  CM_INTP_1 "interpolation table"
  TAB_INTP "%6.2" "Nm" "Torque"
  COMPU_TAB 3 2
    0   0.0
    100 50.0
    200 100.0
/end COMPU_METHOD
```

---

## 六、注意事项

1. **向后兼容**：现有通过 f/offset 自动管理 COMPU_METHOD 的逻辑不改动，新编辑器是补充
2. **命名冲突**：新建 COMPU_METHOD 时检查名称是否已存在
3. **删除保护**：删除前检查是否有变量引用，有则提示但不强制阻止
4. **大文件性能**：列表和解析基于已加载的 content 字符串，不做额外文件 I/O
5. **TAB_VERB 数值类型**：原始值统一用整数索引（ASAM 规范允许浮点，但实际多为枚举整数）
6. **保留 NO_COMPU_METHOD**：不解析也不在列表中显示这个特殊项
