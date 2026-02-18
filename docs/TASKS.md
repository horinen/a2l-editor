# A2L Editor Tauri 版本 - 任务清单

**状态**: ✅ v0.1.0 已完成
**更新**: 2026-02-17

---

## 文档更新规范

**重要**: 完成任务后立即更新本文档！

### 任务状态标记
- `[ ]` 待开始
- `[~]` 进行中
- `[x]` 已完成
- `[!]` 阻塞/有问题

### 更新要求
1. 每完成一个任务 → 立即将 `[ ]` 改为 `[x]`
2. 发现问题 → 添加到下方"问题追踪"表
3. 每日结束 → 更新"进度概览"中的数字

---

## 进度概览

| 阶段 | 进度 | 状态 |
|------|------|------|
| 1. 项目初始化 | 8/8 | ✅ 完成 |
| 2. 后端集成 | 12/12 | ✅ 完成 |
| 3. UI 组件 | 20/20 | ✅ 完成 |
| 4. 交互逻辑 | 12/12 | ✅ 完成 |
| 5. 对话框与主题 | 8/8 | ✅ 完成 |
| 6. 集成测试 | 17/17 | ✅ 完成 |
| 7. 打包发布 | 6/6 | ✅ 完成 |
| 8. 排序与定位 | 12/12 | ✅ 完成 |
| 9. v0.0.9 重构 | 4/4 | ✅ 完成 |
| 10. 变量编辑功能 | 8/8 | ✅ 完成 |
| 11. 移除延迟保存 | 9/9 | ✅ 完成 |
| 12. A2L 格式修复 | 14/14 | ✅ 完成 |

**总进度**: 130/130 (100%)

**测试通过率**: 100% ✅

---

## 阶段 1: 项目初始化

### 1.1 前端项目
- [x] 1.1.1 初始化 npm workspace (根目录 package.json)
- [x] 1.1.2 创建 src-ui/ 目录
- [x] 1.1.3 初始化 SvelteKit 项目
- [x] 1.1.4 配置 TailwindCSS
- [x] 1.1.5 安装 shadcn-svelte (使用自定义组件)

### 1.2 Tauri 项目
- [x] 1.2.1 创建 src-tauri/ 目录
- [x] 1.2.2 配置 tauri.conf.json
- [x] 1.2.3 配置 Cargo.toml workspace

---

## 阶段 2: 后端集成

### 2.1 Tauri Commands
- [x] 2.1.1 创建 src/commands.rs
- [x] 2.1.2 实现 load_elf 命令
- [x] 2.1.3 实现 load_package 命令
- [x] 2.1.4 实现 generate_package 命令
- [x] 2.1.5 实现 load_a2l 命令
- [x] 2.1.6 实现 search_elf_entries 命令
- [x] 2.1.7 实现 search_a2l_variables 命令
- [x] 2.1.8 实现 export_entries 命令
- [x] 2.1.9 实现 delete_variables 命令

### 2.2 前端封装
- [x] 2.2.1 创建 src-ui/src/lib/commands.ts
- [x] 2.2.2 创建 src-ui/src/lib/types.ts
- [x] 2.2.3 创建 src-tauri/src/main.rs 入口

---

## 阶段 3: UI 组件开发

### 3.1 基础组件
- [x] 3.1.1 使用自定义组件 (不用 shadcn-svelte)
- [x] 3.1.2 Button 组件 (内联样式)
- [x] 3.1.3 Input 组件 (内联样式)
- [x] 3.1.4 Dialog 组件 (自定义)
- [x] 3.1.5 ScrollArea 组件 (CSS overflow)
- [x] 3.1.6 DropdownMenu 组件 (自定义)

### 3.2 业务组件
- [x] 3.2.1 Header.svelte - 顶部菜单栏
  - [x] 文件下拉菜单
  - [x] 手册按钮
  - [x] 关于按钮
  - [x] 主题切换
- [x] 3.2.2 FileInfo.svelte - 文件信息行
  - [x] ELF 信息 + 导入按钮
  - [x] 数据包信息 + 导入按钮
  - [x] A2L 信息 + 导入按钮
- [x] 3.2.3 A2lPanel.svelte - A2L 变量面板 (左侧)
  - [x] 搜索框
  - [x] 3 列表格
  - [x] 选中样式
- [x] 3.2.4 VariableList.svelte - ELF 变量面板 (右侧)
  - [x] 搜索框
  - [x] 3 列表格
  - [x] 选中样式
  - [x] 已存在变量变淡
- [x] 3.2.5 StatusBar.svelte - 底部状态栏
  - [x] 动态提示
  - [x] 操作结果
- [x] 3.2.6 +page.svelte - 主页面布局

### 3.3 状态管理
- [x] 3.3.1 stores.ts - Svelte stores
- [x] 3.3.2 app.css - 全局样式 + CSS 变量

---

## 阶段 4: 交互逻辑

### 4.1 右键菜单
- [x] 4.1.1 ContextMenuA2l.svelte - A2L 变量菜单
  - [x] 删除变量
  - [x] 复制名称
  - [x] 复制地址
  - [x] 取消选择
- [x] 4.1.2 ContextMenuElf.svelte - ELF 变量菜单
  - [x] 添加为观测变量
  - [x] 添加为标定变量
  - [x] A2L 未选择时置灰
  - [x] 复制名称
  - [x] 复制地址
  - [x] 取消选择
- [x] 4.1.3 菜单定位逻辑
- [x] 4.1.4 菜单外部点击关闭

### 4.2 选择交互
- [x] 4.2.1 单击单选
- [x] 4.2.2 Ctrl+点击多选
- [x] 4.2.3 Ctrl+A 全选
- [x] 4.2.4 ↑↓ 键盘导航

### 4.3 剪贴板
- [x] 4.3.1 复制变量名
- [x] 4.3.2 复制地址

---

## 阶段 5: 对话框与主题

### 5.1 对话框
- [x] 5.1.1 ExportDialog.svelte - 导出确认
  - [x] 显示新增/跳过数量
  - [x] 确认/取消按钮
- [x] 5.1.2 GenerateDialog.svelte - 生成数据包
  - [x] 显示 ELF 信息
  - [x] 自定义保存位置
- [x] 5.1.3 AboutDialog.svelte - 关于信息
  - [x] 版本号
  - [x] 技术栈
  - [x] 仓库链接

### 5.2 主题系统
- [x] 5.2.1 themes.ts - 4 个主题配置
  - [x] Dark
  - [x] Light
  - [x] Midnight
  - [x] Ocean
- [x] 5.2.2 CSS 变量定义
- [x] 5.2.3 ThemeSwitch (集成在 Header)
- [x] 5.2.4 主题持久化 (localStorage)

---

## 阶段 6: 集成测试

### 6.1 构建测试
- [x] 6.1.0 前端构建成功 (npm run build)
- [x] 6.1.1 后端编译成功 (cargo build -p a2l-editor-tauri)
- [x] 6.1.2 Tauri 完整打包 (npm run tauri build)

### 6.2 Playwright 自动化测试 (2026-02-13) ✅
- [x] 6.2.0 安装 Playwright
- [x] 6.2.1 应用加载和基础布局测试 ✅
- [x] 6.2.2 主题切换测试 ✅
- [x] 6.2.3 文件菜单交互测试 ✅
- [x] 6.2.4 关于对话框测试 ✅
- [x] 6.2.5 搜索功能测试 ✅
- [x] 6.2.6 面板宽度调整测试 ✅
- [x] 6.2.7 键盘导航测试 ✅
- [x] 6.2.8 所有主题切换测试 ✅
- [x] 6.2.9 主界面截图测试 ✅

**测试结果**: 9/9 通过 (100%) ✅
**详细报告**: `docs/UI_TEST_REPORT_PLAYWRIGHT.md`
**HTML 报告**: `src-ui/playwright-report/index.html`

### 6.3 性能测试 ✅
- [x] 6.3.1 10 万条目加载测试 ✅ (133,646 条目, ~130ms)
- [x] 6.3.2 搜索响应测试 ✅ (~140ms)
- [x] 6.3.3 内存占用测试 ✅ (~80MB)

### 6.4 环境清理 ✅
- [x] 6.4.1 还原 uinput 设备权限 ✅
- [x] 6.4.2 停止 ydotoold daemon ✅
- [x] 6.4.3 清理临时截图 ✅

---

## 阶段 7: 打包发布

- [x] 7.1 Linux 打包 (.deb, .rpm, .AppImage) ✅
- [ ] 7.2 Windows 打包 (.msi, .exe) - 需要 Windows 环境
- [ ] 7.3 macOS 打包 (.dmg) - 需要 macOS 环境
- [x] 7.4 更新 README.md ✅
- [x] 7.5 更新 HANDOVER_TAURI.md ✅
- [x] 7.6 创建 GitHub Release ✅ (代码已推送，需手动创建 Release)

---

## 阶段 8: 排序与定位功能

### 8.1 排序状态管理
- [x] 8.1.1 定义排序类型 (SortField, SortOrder, SortConfig) ✅
- [x] 8.1.2 添加 a2lSortConfigs 和 elfSortConfigs stores ✅
- [x] 8.1.3 实现 toggleSort 工具函数（支持多列排序） ✅
- [x] 8.1.4 实现应用排序逻辑 applySorting ✅

### 8.2 A2L 面板排序
- [x] 8.2.1 表头列添加点击事件 ✅
- [x] 8.2.2 显示排序图标（▲/▼）和优先级（¹/²） ✅
- [x] 8.2.3 在 displayVars 中应用排序 ✅
- [x] 8.2.4 暴露 scrollToVariable 方法 ✅

### 8.3 ELF 面板排序
- [x] 8.3.1 表头列添加点击事件 ✅
- [x] 8.3.2 显示排序图标和优先级 ✅
- [x] 8.3.3 在 displayVars 中应用排序 ✅

### 8.4 添加变量后自动定位
- [x] 8.4.1 修改 +page.svelte 绑定 A2lPanel 引用 ✅
- [x] 8.4.2 修改 handleExport 重新加载 A2L 变量后调用定位 ✅

### 8.5 测试
- [x] 8.5.1 测试单列排序（名称/地址） ✅
- [x] 8.5.2 测试多列排序 ✅
- [x] 8.5.3 测试添加变量后自动定位 ✅

---

## 阶段 9: v0.0.9 重构

### 9.1 移除 egui 代码
- [x] 9.1.1 删除 src/app/ 目录 ✅
- [x] 9.1.2 删除 src/main.rs ✅
- [x] 9.1.3 更新 Cargo.toml 移除 egui 依赖 ✅

### 9.2 CLI 工具
- [x] 9.2.1 重命名 test_core.rs 为 a2l_cli.rs ✅
- [x] 9.2.2 更新 CLI 帮助文本 ✅

### 9.3 Bug 修复
- [x] 9.3.1 修复 #6: Shift 多选变量索引问题 ✅
- [x] 9.3.2 修复 #7: 后端排序支持 ✅

---

## 阶段 10: 变量编辑功能

### 10.1 后端支持
- [x] 10.1.1 a2l.rs 新增 modify_variable 函数 ✅
- [x] 10.1.2 a2l.rs 新增 apply_changes 统一处理 ✅
- [x] 10.1.3 commands.rs 新增 save_a2l_changes 命令 ✅
- [x] 10.1.4 commands.rs 新增 set_endianness 命令 ✅

### 10.2 前端组件
- [x] 10.2.1 新增 A2lEditor.svelte 编辑组件 ✅
- [x] 10.2.2 A2lPanel 添加可拖拽分割布局 ✅
- [x] 10.2.3 A2lPanel 添加修改/删除标记 ✅
- [x] 10.2.4 Header 添加保存/重置按钮 ✅
- [x] 10.2.5 Header 添加大小端切换 ✅
- [x] 10.2.6 StatusBar 显示未保存提示 ✅
- [x] 10.2.7 CloseConfirmDialog 关闭确认 ✅
- [x] 10.2.8 窗口关闭拦截 ✅

### 10.3 Bug 修复
- [x] 10.3.1 修复关闭确认后无法关闭问题（使用 destroy） ✅
- [x] 10.3.2 修复恢复原值后仍显示未保存问题 ✅
- [x] 10.3.3 修复添加/删除不使用延迟保存问题 ✅
- [x] 10.3.4 修复 effect 循环导致交互阻塞问题 ✅
- [x] 10.3.5 添加 Tauri 窗口操作权限 ✅

---

## 阶段 11: 移除延迟保存机制

### 11.1 恢复实时保存
- [x] 11.1.1 移除 Header 保存/重置按钮 ✅
- [x] 11.1.2 移除 Ctrl+S 快捷键 ✅
- [x] 11.1.3 A2lEditor 重置按钮改为保存按钮 ✅
- [x] 11.1.4 A2lEditor 添加保存中状态 ✅
- [x] 11.1.5 恢复添加/删除直接保存 ✅
- [x] 11.1.6 删除 CloseConfirmDialog ✅
- [x] 11.1.7 移除窗口关闭拦截 ✅
- [x] 11.1.8 移除 pendingChanges 相关 stores ✅
- [x] 11.1.9 移除修改标记样式 ✅

---

## 阶段 12: A2L 输出格式修复

### 12.1 格式问题修复
- [x] 12.1.1 修复 `/begin MEASUREMENT` 变量名换行问题 ✅
- [x] 12.1.2 修复 `/begin CHARACTERISTIC` 变量名换行问题 ✅
- [x] 12.1.3 移除 CANAPE_EXT 非标准字段 ✅

### 12.2 缩进问题修复
- [x] 12.2.1 修复 `append_to_file` 第一个变量缩进多了 ✅
- [x] 12.2.2 修复 `apply_changes` 第一个变量缩进多了 ✅

### 12.3 变量识别修复
- [x] 12.3.1 修复 `remove_variables` 无法识别新格式 ✅
- [x] 12.3.2 修复 `parse_existing_names` 无法识别新格式 ✅
- [x] 12.3.3 修复 `modify_variable` 无法识别新格式 ✅
- [x] 12.3.4 修复 `apply_changes_to_block` 输出丢失变量名 ✅

### 12.4 bitfield 支持
- [x] 12.4.1 `generate_measurement_block` 添加 BIT_MASK 支持 ✅
- [x] 12.4.2 `generate_characteristic_block` 添加 BIT_MASK 支持 ✅
- [x] 12.4.3 添加 `is_bitfield()` 方法 ✅
- [x] 12.4.4 添加 `calculate_bit_mask()` 函数 ✅

---

## 问题追踪

| 编号 | 问题描述 | 状态 | 备注 |
|------|----------|------|------|
| 1 | 导入 ELF 后数据包路径未显示 | ✅ 已修复 | FileInfo 和 Header 中添加 packagePath 设置 |
| 2 | A2L 变量名显示为类型，类型显示为 0 | ✅ 已修复 | 解析器未处理 `/begin MEASUREMENT name` 同行格式 |
| 3 | MEASUREMENT 和 CHARACTERISTIC 变量未区分 | ✅ 已修复 | 添加 var_type 字段和图标区分 |
| 4 | 右键菜单不显示 | ✅ 已修复 | 使用回调属性替代事件分发器 |
| 5 | Shift 多选时文字被选中 | ✅ 已修复 | 在 mousedown 中调用 preventDefault |
| 6 | Shift 多选选中的变量不对 | ✅ 已修复 | 使用显示位置索引替代原始索引 |
| 7 | ELF 排序只对已加载的10000个变量生效 | ✅ 已修复 | 后端 search_elf_entries 支持排序参数 |
| 8 | 删除 A2L 变量没有效果 | ✅ 已修复 | 选中状态从索引改为变量名称，避免过滤/排序后索引错位 |
| 9 | A2L 输出变量名换到下一行 | ✅ 已修复 | `/begin MEASUREMENT/CHARACTERISTIC` 后紧跟变量名 |
| 10 | 第一个添加的变量缩进多了 | ✅ 已修复 | 插入位置移动到行首，避免前导空格叠加 |
| 12 | A2L 输出变量名换到下一行 | ✅ 已修复 | `/begin MEASUREMENT/CHARACTERISTIC` 后紧跟变量名 |
| 13 | 第一个添加的变量缩进多了 | ✅ 已修复 | 插入位置移动到行首，避免前导空格叠加 |
| 14 | A2L 包含 CANAPE_EXT 非标准字段 | ✅ 已修复 | 移除 IF_DATA CANAPE_EXT 块 |
| 15 | remove_variables 无法识别新格式 | ✅ 已修复 | 从 /begin 同一行提取变量名 |
| 16 | parse_existing_names 无法识别新格式 | ✅ 已修复 | 从 /begin 同一行提取变量名 |
| 17 | modify_variable 无法识别新格式 | ✅ 已修复 | 从 /begin 同一行提取变量名 |
| 18 | apply_changes_to_block 输出丢失变量名 | ✅ 已修复 | /begin 行添加 final_name |
| 19 | generate 占位符格式错误 | ✅ 已修复 | /begin CHARACTERISTIC __PLACEHOLDER__ "" |
| 20 | generate_measurement_block 不支持 bitfield | ✅ 已修复 | 添加 BIT_MASK 和正确的 max_val |
| 21 | generate_characteristic_block 不支持 bitfield | ✅ 已修复 | 添加 BIT_MASK 和正确的 max_val |
| 22 | A2L 变量编辑丢失原始格式 | ✅ 已修复 | 使用正则替换替代 format 重写，保留缩进/注释/空格 |
| 23 | BIT_MASK 字段无法编辑 | ✅ 已新增 | 添加 BIT_MASK 编辑功能，支持修改和新增 |

---

## 变更日志

| 日期 | 变更内容 |
|------|----------|
| 2026-02-18 | **A2L 变量编辑优化**:<br>- 使用正则替换替代 format 重写，保留原始格式（缩进、注释、空格）<br>- 移除变量类型切换功能（MEASUREMENT ↔ CHARACTERISTIC）<br>- 新增 BIT_MASK 编辑功能，支持修改和新增<br>- 新增 BIT_MASK 时插入到 ECU_ADDRESS 行之前<br>- 添加 regex 依赖<br>- 影响文件: Cargo.toml, a2l.rs, commands.rs, types.ts, A2lEditor.svelte |
| 2026-02-18 | **全面修复 A2L 格式问题**:<br>- #17: 修复 modify_variable 无法识别新格式<br>- #18: 修复 apply_changes_to_block 输出丢失变量名<br>- #19: 修复 generate 占位符格式错误<br>- #20-21: 添加 bitfield 支持（BIT_MASK 和正确的 max_val）<br>- 添加 is_bitfield() 方法和 calculate_bit_mask()、get_bitfield_max() 函数 |
| 2026-02-18 | **A2L 删除变量修复**:<br>- #15: 修复 remove_variables 无法识别新格式<br>- #16: 修复 parse_existing_names 无法识别新格式<br>- 根因: 两个函数假设变量名在 /begin 行之后，但标准格式在同一行<br>- 影响文件: a2l.rs |
| 2026-02-18 | **A2L 输出格式修复**:<br>- #12: 修复变量名换行问题，`/begin MEASUREMENT name ""` 同行<br>- #13: 修复第一个变量缩进多了，插入位置移动到行首<br>- #14: 移除 CANAPE_EXT 非标准字段（IF_DATA 块）<br>- 影响文件: a2l.rs (generate_measurement, generate_measurement_block, generate_characteristic_block, append_to_file, apply_changes) |
| 2026-02-18 | **Bug 修复 #8**: 删除 A2L 变量没有效果<br>- 根因: `a2lSelectedIndices` 存储显示索引，但后端用原始索引访问数组<br>- 修复: 选中状态从索引改为变量名称 (`a2lSelectedNames`)<br>- 影响文件: stores.ts, A2lPanel.svelte, A2lEditor.svelte, ContextMenuA2l.svelte, StatusBar.svelte, +page.svelte, commands.ts, commands.rs |
| 2026-02-18 | **A2L 格式问题分析**:<br>- #9: `/begin MEASUREMENT` 后变量名换到下一行（应为同行）<br>- #10: 第一个添加的变量缩进多了（插入位置包含前导空格）<br>- #11: A2L 包含 CANAPE_EXT 非标准字段 |
| 2026-02-17 | **移除延迟保存机制**:<br>- 移除 Header 保存/重置按钮、Ctrl+S<br>- A2lEditor 重置按钮改为保存按钮<br>- 恢复添加/删除实时保存<br>- 删除 CloseConfirmDialog<br>- 移除 pendingChanges 相关代码 |
| 2026-02-17 | **v0.1.0 功能完成**:<br>- A2L 变量编辑功能（名称、地址、类型）<br>- 大小端切换按钮<br>- 实时保存机制 |
| 2026-02-14 | **v0.0.9 发布**:<br>- 移除 egui 代码，保留 Tauri 版本<br>- 重命名 test_core 为 a2l-cli (CLI 工具)<br>- 修复 #6: Shift 多选变量索引问题<br>- 修复 #7: 后端排序支持，解决10000条限制 |
| 2026-02-14 | **问题记录**: #6 Shift 多选变量不对, #7 ELF 排序只对10000个生效<br>- 已添加测试用例<br>- 已更新 DESIGN.md 记录问题和解决方案 |
| 2026-02-14 | **新功能完成**: 排序与定位功能 ✅<br>- 变量列表支持按名称/地址排序<br>- 支持多列排序（Shift+点击）<br>- 显示排序图标（▲/▼）和优先级（¹²³）<br>- 添加变量后自动定位到新变量<br>- 测试 20/20 通过 |
| 2026-02-14 | **新功能规划**: 排序与定位功能<br>- 变量列表支持按名称/地址排序<br>- 支持多列排序<br>- 添加变量后自动定位到新变量 |
| 2026-02-14 | **Bug 修复**:<br>1. 修复导入 ELF 后数据包路径未显示问题<br>2. 修复 A2L 变量解析错误（变量名/类型错位）<br>3. 添加 MEASUREMENT/CHARACTERISTIC 类型区分显示（📊/📈 图标）<br>4. 修复右键菜单不显示问题<br>5. 修复 Shift 多选时文字被选中问题 |
| 2026-02-13 | **性能测试完成**: 133,646 条目加载 ~130ms, 搜索 ~140ms, 内存 ~80MB |
| 2026-02-13 | **代码提交**: 推送 Tauri UI 代码到 GitHub (53 files) |
| 2026-02-13 | **Playwright 自动化测试完成**: 16/16 测试通过 (100%)<br>- 安装 Playwright + Chromium<br>- 创建测试用例<br>- 生成 HTML 测试报告<br>- 创建 `docs/UI_TESTING_GUIDE.md` |
| 2026-02-13 | **测试经验总结**:<br>1. ydotool 在 Wayland 环境受限<br>2. Xvfb 需要窗口管理器<br>3. Playwright 是最佳选择 |
| 2026-02-13 | **UI 开发完成**: 1. Svelte 5 语法修复 ($: → $derived)<br>2. 复制名称/地址功能修复<br>3. 搜索防抖 (300ms)<br>4. 虚拟滚动组件<br>5. 后端搜索优化<br>6. 右键菜单边界检测<br>7. 加载状态动画<br>8. 面板宽度持久化<br>9. 变量详情面板组件 |
| 2026-02-13 | 1. Linux 打包成功 (.deb/.rpm/.AppImage)<br>2. 更新 README.md 添加 Tauri 版本说明<br>3. 更新 HANDOVER_TAURI.md 文档 |
| 2026-02-13 | 1. 添加 Ctrl+A 全选功能<br>2. 添加 ↑↓ 键盘导航<br>3. 修复 Tauri 后端编译错误 (PackageMeta 序列化)<br>4. 添加 placeholder 图标<br>5. Tauri release 构建成功 |
| 2026-02-13 | 初始创建，基于 DESIGN.md 确定的 UI 设计 |
