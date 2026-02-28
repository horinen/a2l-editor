# A2L Editor - Excel 批量导入变量功能计划

## 项目信息

| 项目 | 信息 |
|------|------|
| 项目名称 | Excel 批量导入变量功能 |
| 开始日期 | 待定 |
| 完成日期 | 待定 |
| 状态 | 📋 规划中 |

---

## 1. 功能概述

| 功能 | 说明 |
|------|------|
| 导入 Excel | 从 Excel 批量导入/更新 A2L 变量 |
| 导出模板 | 生成带示例行的 Excel 模板文件 |

---

## 2. Excel 文件格式

### 2.1 列定义

| 列名 | 说明 | 填入 A2L 位置 | 示例 |
|------|------|--------------|------|
| 名称 | 变量名 | `/begin MEASUREMENT {名称}` 或 `/begin CHARACTERISTIC {名称}` | `Engine_RPM` |
| link | 符号链接 | `SYMBOL_LINK "{link}" 0` | `EngineSpeed` |
| 变量类型 | 观测/标定 | 决定使用 MEASUREMENT 还是 CHARACTERISTIC | `观测` |
| 转换关系 | 预留 | 暂不处理 | - |

### 2.2 变量类型映射

| Excel 值 | A2L 块类型 |
|---------|-----------|
| 观测 | `/begin MEASUREMENT` |
| 标定 | `/begin CHARACTERISTIC` |

### 2.3 导入匹配逻辑

| 项目 | 说明 |
|------|------|
| **匹配键** | Excel **link 列** → 在 ELF 中查找变量 |
| **获取信息** | 地址、数据类型（从 ELF 匹配的变量获取） |
| **写入名称** | Excel **名称列** → A2L 变量名 |
| **写入 link** | Excel **link 列** → SYMBOL_LINK |

### 2.4 示例 Excel 内容

| 名称 | link | 变量类型 | 转换关系 |
|------|------|----------|----------|
| 发动机转速 | EngineSpeed | 观测 | |
| 冷却液温度 | CoolantTemp | 观测 | |
| 燃油喷射量 | FuelRate | 标定 | |

---

## 3. 生成的 A2L 内容示例

**输入 Excel**:

| 名称 | link | 变量类型 |
|------|------|----------|
| 发动机转速 | EngineSpeed | 观测 |

**输出 A2L**（假设 EngineSpeed 在 ELF 中地址为 0x20000000，类型为 ULONG）:

```a2l
/begin MEASUREMENT 发动机转速 ""
  ULONG NO_COMPU_METHOD 0 0 0 4294967295
  ECU_ADDRESS 0x20000000
  ECU_ADDRESS_EXTENSION 0x0
  FORMAT "%10.0"
  SYMBOL_LINK "EngineSpeed" 0
/end MEASUREMENT
```

---

## 4. 功能详细设计

### 4.1 导入 Excel

**入口**: A2L 面板 → 📥 导入按钮

**前置检查**:

| 条件 | 不满足时 |
|------|---------|
| 已加载 ELF | 提示"请先导入 ELF 文件" |
| 已选择 A2L | 提示"请先选择目标 A2L 文件" |

**处理流程**:

```
1. 选择 .xlsx 文件
2. 解析 Excel，验证列名
3. 遍历数据行：
   - 校验名称、link 非空
   - 校验变量类型为"观测"或"标定"
   - 用 link 在 ELF 中查找变量
   - 找到 → 记录导入
   - 未找到 → 跳过，计入未匹配
4. 批量写入 A2L（覆盖模式：先删后加）
5. 显示结果
```

**覆盖逻辑**:

- 如果 A2L 中已存在同名变量 → 先删除，再添加新变量
- 如果 A2L 中不存在 → 直接添加

### 4.2 导出模板

**入口**: A2L 面板 → 📤 模板按钮

**输出内容**:

- 标题行：名称、link、变量类型、转换关系
- 一行示例数据（固定内容）
- 变量类型列设置下拉选项（观测、标定）

**示例行内容**:

| 名称 | link | 变量类型 | 转换关系 |
|------|------|----------|----------|
| VariableName | SymbolName | 观测 | |

---

## 5. 界面设计

```
┌──────────────── A2L 变量 ────────────────┐
│ 🔍 [搜索...] [✖]   📥导入  📤模板  ➕   │
└──────────────────────────────────────────┘
```

| 按钮 | 功能 |
|------|------|
| 📥 导入 | 选择 Excel 文件，批量导入变量 |
| 📤 模板 | 导出 Excel 模板文件 |
| ➕ 新增 | 手动添加单个变量（已有功能） |

---

## 6. 错误处理与提示

### 6.1 导入错误

| 情况 | 处理 |
|------|------|
| 缺少"名称"列 | 提示错误，中止导入 |
| 缺少"link"列 | 提示错误，中止导入 |
| 缺少"变量类型"列 | 提示错误，中止导入 |
| 名称或 link 为空 | 跳过该行 |
| 变量类型值无效 | 跳过该行 |
| link 在 ELF 中不存在 | 跳过，计入未匹配 |

### 6.2 结果提示

| 结果 | 状态栏提示 |
|------|-----------|
| 全部成功 | `✅ 已导入 50 个变量` |
| 部分成功 | `✅ 已导入 45 个变量，5 个未在 ELF 中找到` |
| 全部失败 | `⚠️ 没有可导入的变量` |

---

## 7. 技术实现要点

### 7.1 后端新增

| 项目 | 说明 |
|------|------|
| `symbol_link` 字段 | `VariableChanges` 和 `VariableEditInput` 添加此字段 |
| 生成函数修改 | `generate_measurement_block_with_compu` 和 `generate_characteristic_block_with_compu` 支持自定义 SYMBOL_LINK |

### 7.2 前端新增

| 项目 | 说明 |
|------|------|
| `xlsx` 库 | 用于解析和生成 Excel 文件 |
| `ImportExcelDialog.svelte` | 导入对话框组件（处理文件选择、解析、导入） |
| 导出模板函数 | 生成带下拉选项的 Excel 文件 |

### 7.3 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `src-ui/package.json` | 添加 xlsx 依赖 |
| `src/lib/types.rs` | VariableChanges 添加 symbol_link |
| `src/lib/a2l.rs` | 生成函数支持 symbol_link 参数 |
| `src-tauri/src/commands.rs` | VariableEditInput 添加 symbol_link |
| `src-ui/src/lib/types.ts` | 添加 Excel 相关类型 |
| `src-ui/src/lib/commands.ts` | 批量导入逻辑 |
| `src-ui/src/lib/components/A2lPanel.svelte` | 添加导入/导出按钮 |
| `src-ui/src/lib/components/ImportExcelDialog.svelte` | 新建导入对话框 |

---

## 8. 任务清单

### 阶段 1: 后端支持 SYMBOL_LINK (预计 1 小时)

- [ ] 1.1 `VariableChanges` 添加 `symbol_link: Option<String>` 字段
- [ ] 1.2 `A2lEntryInfo` 添加 `symbol_link` 字段
- [ ] 1.3 修改 `generate_measurement_block_with_compu` 支持 symbol_link 参数
- [ ] 1.4 修改 `generate_characteristic_block_with_compu` 支持 symbol_link 参数
- [ ] 1.5 修改 `apply_changes_to_block` 支持 SYMBOL_LINK 替换
- [ ] 1.6 `VariableEditInput` 添加 `symbol_link` 字段

### 阶段 2: 前端导入功能 (预计 1.5 小时)

- [ ] 2.1 安装 `xlsx` 依赖
- [ ] 2.2 创建 `ExcelImportRow` 类型定义
- [ ] 2.3 创建 `ImportExcelDialog.svelte` 组件
- [ ] 2.4 实现 Excel 文件选择和解析
- [ ] 2.5 实现从 ELF 匹配变量信息（用 link 列匹配）
- [ ] 2.6 实现批量覆盖逻辑（先删除再添加）
- [ ] 2.7 在 A2lPanel 添加导入/导出模板按钮
- [ ] 2.8 实现导出 Excel 模板功能

### 阶段 3: 测试验证 (预计 0.5 小时)

- [ ] 3.1 测试新建变量导入
- [ ] 3.2 测试覆盖已有变量
- [ ] 3.3 测试 SYMBOL_LINK 正确写入
- [ ] 3.4 测试变量类型识别（观测/标定）
- [ ] 3.5 测试导出模板功能
- [ ] 3.6 运行 `cargo test` 和 `npm run check`

---

## 9. 确认清单

| 项目 | 确认 |
|------|------|
| Excel 列名：名称、link、变量类型、转换关系 | ✅ |
| 变量类型使用中文：观测、标定 | ✅ |
| 用 link 列匹配 ELF 获取地址和类型 | ✅ |
| 名称列作为 A2L 变量名 | ✅ |
| link 列作为 SYMBOL_LINK | ✅ |
| 名称和 link 可以相同也可以不同 | ✅ |
| 重复变量名采用覆盖模式 | ✅ |
| 导出模板包含一行示例 | ✅ |
| 导入时无预览，直接导入 | ✅ |
| 仅支持 .xlsx 格式 | ✅ |
| 按钮位于 A2L 面板搜索栏右侧 | ✅ |
