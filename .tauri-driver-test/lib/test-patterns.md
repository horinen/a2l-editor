# A2L Editor 测试模式参考

## 一、Tauri 命令测试模式

### 1.1 枚举设置类命令
适用于：`set_endianness` 等有固定选项的命令

**必须覆盖的测试场景**：
| 场景 | 测试内容 | 示例 |
|------|----------|------|
| 正常值 A | 设置第一个有效值 | `set_endianness('little')` → success |
| 正常值 B | 设置第二个有效值 | `set_endianness('big')` → success |
| 无效值 | 设置不存在的值 | `set_endianness('invalid')` → error |

**步骤模板**：
```json
[
  { "action": "execute_script", "script": "return window.__TAURI__.core.invoke('set_endianness', { endianness: 'little' }).then(() => 'success').catch(e => 'error: ' + e);" },
  { "action": "assert_result_equals", "expect": "success" }
]
```

### 1.2 文件加载类命令
适用于：`load_elf`, `load_a2l`, `load_package`

**必须覆盖的测试场景**：
| 场景 | 测试内容 |
|------|----------|
| 文件不存在 | 验证返回明确的错误信息 |
| 正常加载 | 验证返回正确的元信息 |
| 数据包不存在 | 提示需要先生成 |

### 1.3 搜索/查询类命令
适用于：`search_elf_entries`, `search_a2l_variables`, `get_elf_count`

**必须覆盖的测试场景**：
| 场景 | 测试内容 |
|------|----------|
| 未初始化 | 返回错误或空结果 |
| 空查询 | 返回全部或默认结果 |
| 精确匹配 | 返回匹配结果 |

---

## 二、UI 交互测试模式

### 2.1 状态切换按钮
适用于：字节序切换、主题切换

**必须覆盖的测试场景**：
| 场景 | 测试内容 |
|------|----------|
| 状态变化 | 点击后文本实际变化 |
| 循环切换 | 多次点击能循环回初始状态 |

**步骤模板**：
```json
[
  { "action": "execute_script", "script": "return document.querySelector('.endianness-btn')?.textContent?.trim();" },
  { "action": "store_result", "key": "before" },
  { "action": "execute_script", "script": "document.querySelector('.endianness-btn')?.click(); return true;" },
  { "action": "wait", "duration": 300 },
  { "action": "execute_script", "script": "return document.querySelector('.endianness-btn')?.textContent?.trim();" },
  { "action": "assert_result_changed", "from": "stored:before" }
]
```

### 2.2 菜单操作
适用于：文件菜单等下拉菜单

**注意事项**：
- 使用单脚本模式，避免菜单在两次脚本执行之间关闭
- 点击和验证操作放在同一个脚本中

**步骤模板**：
```json
[
  { "action": "execute_script", "script": "const btn = document.querySelector('.menu-btn'); btn.click(); return document.querySelector('.menu') !== null;" },
  { "action": "assert_result_equals", "expect": true }
]
```

### 2.3 对话框
适用于：关于对话框、帮助对话框、生成对话框、导出对话框

**必须覆盖的测试场景**：
| 场景 | 测试内容 |
|------|----------|
| 打开 | 触发后对话框显示 |

---

## 三、错误处理测试

### 3.1 资源不存在
适用于：所有文件操作命令

**必须覆盖**：
- 不存在的 ELF 文件
- 不存在的数据包
- 不存在的 A2L 文件

---

## 四、已知 Bug 回归测试

（待添加）

---

## 五、项目特定注意事项

1. **主题检测**：使用 `document.documentElement.classList` 检测主题类名（light/midnight/ocean），dark 是默认主题无类名
2. **主题按钮选择器**：使用 `button[title="切换主题"]` 而非 `.theme-btn`
3. **菜单关闭问题**：Svelte 使用 `<svelte:window onclick={closeMenu} />`，WebDriver 在两次脚本执行之间可能触发关闭，需要使用单脚本模式
4. **长时间操作**：`generate_package` 需要设置 `timeout: 600000`（10分钟），457MB ELF 约需 30 分钟

---

## 六、测试文件

| 文件 | 路径 | 用途 |
|------|------|------|
| sample.elf | `/home/hori/project/sample.elf` | 用于 generate_package 测试 |
| sample.a2l | `/home/hori/project/sample.a2l` | 用于 load_a2l 测试 |
