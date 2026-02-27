# A2L Editor - 转化系数编辑功能计划

## 项目信息

| 项目 | 信息 |
|------|------|
| 项目名称 | A2L 变量转化系数编辑功能 |
| 开始日期 | 待定 |
| 完成日期 | 待定 |
| 状态 | 📋 规划中 |

---

## 背景分析

### 当前问题

A2L 变量目前只支持 `NO_COMPU_METHOD`（无转换系数），无法定义原始值到物理值的转换关系。

用户需求：
- 为 A2L 变量添加转化系数编辑功能（F 斜率和 OFFSET 偏移）
- 支持设置物理单位（如 °C, rpm, ms）
- 像 ASAP Studio 一样在编辑区域直接输入 F/OFFSET/Unit
- 相同系数的变量共享同一个 COMPU_METHOD 定义

### A2L COMPU_METHOD 格式

```a2l
/begin COMPU_METHOD
  CM_F0_5_O10 "y = 0.5 * x + 10.0"
  LINEAR "%10.4" "°C"
  COEFFS 0.5 10.0 0.0 0.0 0.0 0.0
/end COMPU_METHOD
```

转换公式：`物理值 = F × 原始值 + OFFSET`

---

## 功能目标

| 功能 | 说明 |
|------|------|
| F (斜率) | 浮点数输入，如 0.5, 1.0, 0.01 |
| OFFSET (偏移) | 浮点数输入，如 0.0, -273.15, 10.0 |
| Unit (单位) | 字符串输入，如 °C, rpm, ms |
| 默认行为 | 默认使用 NO_COMPU_METHOD，用户填写 F 后才生成 COMPU_METHOD |
| 共享机制 | 相同 F/OFFSET 的变量共享同名 COMPU_METHOD |

### COMPU_METHOD 命名规则

生成格式: `CM_F{f}_O{offset}`

示例：
- F=1.0, OFFSET=0.0 → `CM_F1_O0`
- F=0.5, OFFSET=10.0 → `CM_F0_5_O10`
- F=0.01, OFFSET=-273.15 → `CM_F0_01_ON273_15` (负数用 N 表示)

---

## 技术方案

### 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `src/lib/a2l.rs` | A2lVariable/VariableChanges 添加字段；解析/生成 COMPU_METHOD |
| `src-tauri/src/commands.rs` | VariableInfo/EditInput 添加字段 |
| `src-ui/src/lib/types.ts` | A2lVariable/Edit 添加字段 |
| `src-ui/src/lib/components/A2lEditor.svelte` | 添加 F/OFFSET/Unit 编辑字段 |

### 数据结构变更

**Rust A2lVariable:**
```rust
pub struct A2lVariable {
    pub name: String,
    pub address: Option<String>,
    pub var_type: String,
    pub data_type: String,
    pub bit_mask: Option<String>,
    pub compu_method: Option<String>,  // 新增
    pub f: Option<f64>,                // 新增
    pub offset: Option<f64>,           // 新增
    pub unit: Option<String>,          // 新增
}
```

**TypeScript A2lVariable:**
```typescript
interface A2lVariable {
  name: string;
  address: string | null;
  data_type: string;
  var_type: 'MEASUREMENT' | 'CHARACTERISTIC';
  bit_mask: string | null;
  compu_method: string | null;  // 新增
  f: number | null;             // 新增
  offset: number | null;        // 新增
  unit: string | null;          // 新增
}
```

### 核心算法

**共享 COMPU_METHOD 生成流程:**
1. 收集所有变更中需要生成的 F/OFFSET 组合
2. 检查文件中是否已存在相同系数的 COMPU_METHOD
3. 生成新的 COMPU_METHOD 块，插入到 `/end MODULE` 前
4. 变量块中引用对应的 COMPU_METHOD 名称

---

## 任务清单

### 阶段 1: 后端核心 (预计 2-3 小时)

- [ ] 1.1 修改 A2lVariable 结构体，添加 compu_method, f, offset, unit 字段
- [ ] 1.2 修改 VariableChanges 结构体，添加相同字段
- [ ] 1.3 实现 `parse_compu_methods()` 解析文件中所有 COMPU_METHOD
- [ ] 1.4 修改 `parse_variable_block()` 解析变量的 COMPU_METHOD 引用和系数
- [ ] 1.5 实现 `generate_compu_method_name(f, offset)` 生成共享名称
- [ ] 1.6 实现 `generate_compu_method_block(name, f, offset, unit)` 生成块
- [ ] 1.7 修改 `generate_measurement_block()` 支持 COMPU_METHOD 参数
- [ ] 1.8 修改 `generate_characteristic_block()` 支持 COMPU_METHOD 参数
- [ ] 1.9 修改 `apply_changes_to_block()` 支持 COMPU_METHOD 名称替换
- [ ] 1.10 修改 `apply_changes()` 实现共享 COMPU_METHOD 逻辑

### 阶段 2: Tauri 命令层 (预计 0.5 小时)

- [ ] 2.1 修改 `VariableInfo` 添加 compu_method, f, offset, unit 字段
- [ ] 2.2 修改 `VariableEditInput` 添加相同字段
- [ ] 2.3 更新 `save_a2l_changes` 传递新字段

### 阶段 3: 前端修改 (预计 1 小时)

- [ ] 3.1 修改 `types.ts` 中 A2lVariable 添加新字段
- [ ] 3.2 修改 `types.ts` 中 A2lVariableEdit 添加新字段
- [ ] 3.3 修改 `A2lEditor.svelte` 添加 F 输入框
- [ ] 3.4 修改 `A2lEditor.svelte` 添加 OFFSET 输入框
- [ ] 3.5 修改 `A2lEditor.svelte` 添加 Unit 输入框
- [ ] 3.6 修改保存逻辑包含新字段

### 阶段 4: 测试验证 (预计 1 小时)

- [ ] 4.1 测试新建变量时设置 F/OFFSET/Unit
- [ ] 4.2 测试修改已有变量的 F/OFFSET/Unit
- [ ] 4.3 测试相同 F/OFFSET 的变量共享 COMPU_METHOD
- [ ] 4.4 测试解析已有 A2L 文件中的 COMPU_METHOD
- [ ] 4.5 运行 `cargo test` 验证后端功能
- [ ] 4.6 运行 `npm run check` 验证前端类型

---

## 验收标准

- [ ] 变量编辑区域可以输入 F, OFFSET, Unit
- [ ] 保存后 A2L 文件正确生成 COMPU_METHOD 块
- [ ] 相同 F/OFFSET 的变量引用同一个 COMPU_METHOD
- [ ] 可以正确解析已有 A2L 文件中的 COMPU_METHOD
- [ ] 所有现有测试通过 (`cargo test`)
- [ ] 前端类型检查通过 (`npm run check`)

---

## 风险与缓解

| 风险 | 可能性 | 缓解措施 |
|------|--------|----------|
| COEFFS 格式解析错误 | 中 | 增加多种格式测试用例 |
| 共享 COMPU_METHOD 冲突 | 低 | 使用统一的命名规则 |
| 浮点数精度问题 | 低 | 保留合理小数位数 |

---

## 参考资料

- A2L ASAP2 标准文档
- COMPU_METHOD 定义: https://www.canfd.net/a2l.html
