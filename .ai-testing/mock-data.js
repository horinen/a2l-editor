/**
 * Tauri API Mock for E2E Testing
 * 用于在浏览器环境下模拟 Tauri API
 */

// ============================================================
// 类型定义
// ============================================================

/**
 * @typedef {Object} A2lEntry
 * @property {number} index - 条目索引
 * @property {string} full_name - 完整变量名
 * @property {number} address - 内存地址
 * @property {number} size - 数据大小(字节)
 * @property {string} a2l_type - A2L类型(SCALAR, ARRAY等)
 * @property {string} type_name - 数据类型名称
 * @property {number|null} bit_offset - 位偏移(位域)
 * @property {number|null} bit_size - 位大小(位域)
 */

/**
 * @typedef {Object} A2lVariable
 * @property {string} name - 变量名
 * @property {string|null} address - 地址字符串
 * @property {string} data_type - 数据类型
 * @property {'MEASUREMENT'|'CHARACTERISTIC'} var_type - 变量类型
 */

/**
 * @typedef {Object} PackageMeta
 * @property {string} file_name - 文件名
 * @property {string|null} elf_path - ELF文件路径
 * @property {number} entry_count - 条目数量
 * @property {number} created_at - 创建时间戳
 */

/**
 * @typedef {Object} LoadResult
 * @property {PackageMeta} meta - 元信息
 * @property {number} entry_count - 条目数量
 */

/**
 * @typedef {Object} A2lLoadResult
 * @property {string} path - 文件路径
 * @property {number} variable_count - 变量数量
 * @property {string[]} existing_names - 已存在的变量名列表
 */

/**
 * @typedef {Object} ExportResult
 * @property {number} added - 添加数量
 * @property {number} skipped - 跳过数量
 * @property {number} existing - 已存在数量
 */

/**
 * @typedef {Object} SaveResult
 * @property {number} modified - 修改数量
 * @property {number} deleted - 删除数量
 * @property {number} added - 添加数量
 * @property {number} skipped - 跳过数量
 */

// ============================================================
// 模拟数据生成器
// ============================================================

/**
 * 生成模拟 ELF 条目数据
 * @param {number} count - 生成数量
 * @returns {A2lEntry[]}
 */
function generateMockElfEntries(count = 1000) {
  const types = ['SCALAR', 'ARRAY', 'STRUCT'];
  const dataTypes = ['uint8', 'uint16', 'uint32', 'int8', 'int16', 'int32', 'float', 'double'];
  const prefixes = ['app_', 'sys_', 'drv_', 'cal_', 'meas_', 'diag_', 'ctrl_', 'sensor_', 'actuator_'];

  const entries = [];

  for (let i = 0; i < count; i++) {
    const prefix = prefixes[Math.floor(Math.random() * prefixes.length)];
    const baseName = `variable_${String(i).padStart(4, '0')}`;
    const fullName = `${prefix}${baseName}`;

    entries.push({
      index: i,
      full_name: fullName,
      address: 0x20000000 + Math.floor(Math.random() * 0x1000000),
      size: Math.pow(2, Math.floor(Math.random() * 4)), // 1, 2, 4, 8 bytes
      a2l_type: types[Math.floor(Math.random() * types.length)],
      type_name: dataTypes[Math.floor(Math.random() * dataTypes.length)],
      bit_offset: Math.random() > 0.9 ? Math.floor(Math.random() * 32) : null,
      bit_size: Math.random() > 0.9 ? Math.floor(Math.random() * 8) + 1 : null,
    });
  }

  return entries;
}

/**
 * 生成模拟 A2L 变量数据
 * @param {number} count - 生成数量
 * @param {A2lEntry[]} [elfEntries] - ELF条目(用于生成匹配的变量)
 * @returns {A2lVariable[]}
 */
function generateMockA2lVariables(count = 500, elfEntries = null) {
  const dataTypes = ['UBYTE', 'SBYTE', 'UWORD', 'SWORD', 'ULONG', 'SLONG', 'A_UINT64', 'A_INT64', 'FLOAT32_IEEE', 'FLOAT64_IEEE'];
  const varTypes = ['MEASUREMENT', 'CHARACTERISTIC'];

  const variables = [];

  // 如果提供了 ELF 条目，部分变量使用 ELF 条目的名称
  const useElfNames = elfEntries && elfEntries.length > 0;
  const elfNameCount = useElfNames ? Math.min(Math.floor(count * 0.3), elfEntries.length) : 0;

  // 从 ELF 条目创建部分变量
  for (let i = 0; i < elfNameCount; i++) {
    const entry = elfEntries[i];
    variables.push({
      name: entry.full_name,
      address: `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}`,
      data_type: dataTypes[Math.floor(Math.random() * dataTypes.length)],
      var_type: varTypes[Math.floor(Math.random() * varTypes.length)],
    });
  }

  // 创建其余变量
  const prefixes = ['existing_', 'old_', 'legacy_', 'custom_'];
  for (let i = elfNameCount; i < count; i++) {
    const prefix = prefixes[Math.floor(Math.random() * prefixes.length)];
    variables.push({
      name: `${prefix}variable_${String(i).padStart(4, '0')}`,
      address: `0x${(0x20000000 + Math.floor(Math.random() * 0x1000000)).toString(16).toUpperCase().padStart(8, '0')}`,
      data_type: dataTypes[Math.floor(Math.random() * dataTypes.length)],
      var_type: varTypes[Math.floor(Math.random() * varTypes.length)],
    });
  }

  return variables;
}

// ============================================================
// 模拟状态存储
// ============================================================

class MockAppState {
  constructor() {
    /** @type {A2lEntry[]} */
    this.elfEntries = [];

    /** @type {A2lVariable[]} */
    this.a2lVariables = [];

    /** @type {string|null} */
    this.elfPath = null;

    /** @type {string|null} */
    this.a2lPath = null;

    /** @type {Set<string>} */
    this.a2lNames = new Set();

    /** @type {string} */
    this.endianness = 'little';

    /** @type {PackageMeta|null} */
    this.packageMeta = null;

    // 初始化模拟数据
    this.elfEntries = generateMockElfEntries(1000);
    this.a2lVariables = generateMockA2lVariables(500, this.elfEntries);
  }

  /**
   * 模拟加载 ELF 数据包
   * @param {string} path
   * @returns {LoadResult}
   */
  loadPackage(path) {
    this.elfPath = path.replace('.a2ldata', '.elf');
    this.packageMeta = {
      file_name: path.split('/').pop().replace('.a2ldata', '.elf'),
      elf_path: this.elfPath,
      entry_count: this.elfEntries.length,
      created_at: Date.now() - Math.floor(Math.random() * 86400000 * 30),
    };

    return {
      meta: this.packageMeta,
      entry_count: this.elfEntries.length,
    };
  }

  /**
   * 模拟加载 ELF 文件
   * @param {string} path
   * @returns {LoadResult}
   */
  loadElf(path) {
    // 模拟数据包不存在的情况
    if (path.includes('no_package')) {
      throw new Error('数据包不存在，请先生成');
    }

    return this.loadPackage(path + '.a2ldata');
  }

  /**
   * 模拟生成数据包
   * @param {string} elfPath
   * @param {string} [outputPath]
   * @returns {PackageMeta}
   */
  generatePackage(elfPath, outputPath) {
    this.elfPath = elfPath;
    this.packageMeta = {
      file_name: elfPath.split('/').pop(),
      elf_path: elfPath,
      entry_count: this.elfEntries.length,
      created_at: Date.now(),
    };

    return this.packageMeta;
  }

  /**
   * 模拟加载 A2L 文件
   * @param {string} path
   * @returns {A2lLoadResult}
   */
  loadA2l(path) {
    this.a2lPath = path;
    this.a2lNames = new Set(this.a2lVariables.map(v => v.name));

    return {
      path: path,
      variable_count: this.a2lVariables.length,
      existing_names: Array.from(this.a2lNames),
    };
  }

  /**
   * 模拟搜索 ELF 条目
   * @param {string} query
   * @param {number} offset
   * @param {number} limit
   * @param {string} sortField
   * @param {string} sortOrder
   * @returns {A2lEntry[]}
   */
  searchElfEntries(query, offset = 0, limit = 10000, sortField = 'name', sortOrder = 'asc') {
    let results = this.elfEntries;

    if (query) {
      const q = query.toLowerCase();
      results = results.filter(e => e.full_name.toLowerCase().includes(q));
    }

    // 排序
    results = [...results].sort((a, b) => {
      const cmp = sortField === 'address'
        ? a.address - b.address
        : a.full_name.localeCompare(b.full_name);
      return sortOrder === 'desc' ? -cmp : cmp;
    });

    return results.slice(offset, offset + limit);
  }

  /**
   * 获取 ELF 条目总数
   * @returns {number}
   */
  getElfCount() {
    return this.elfEntries.length;
  }

  /**
   * 模拟搜索 A2L 变量
   * @param {string} query
   * @param {number} offset
   * @param {number} limit
   * @returns {A2lVariable[]}
   */
  searchA2lVariables(query, offset = 0, limit = 10000) {
    let results = this.a2lVariables;

    if (query) {
      const q = query.toLowerCase();
      results = results.filter(v => v.name.toLowerCase().includes(q));
    }

    return results.slice(offset, offset + limit);
  }

  /**
   * 模拟导出条目到 A2L
   * @param {number[]} indices
   * @param {string} mode
   * @returns {ExportResult}
   */
  exportEntries(indices, mode) {
    const result = { added: 0, skipped: 0, existing: 0 };

    for (const idx of indices) {
      const entry = this.elfEntries.find(e => e.index === idx);
      if (!entry) continue;

      if (this.a2lNames.has(entry.full_name)) {
        result.existing++;
      } else {
        // 添加到 A2L 变量列表
        const aVar = {
          name: entry.full_name,
          address: `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}`,
          data_type: 'UBYTE',
          var_type: mode === 'measurement' ? 'MEASUREMENT' : 'CHARACTERISTIC',
        };
        this.a2lVariables.push(aVar);
        this.a2lNames.add(entry.full_name);
        result.added++;
      }
    }

    return result;
  }

  /**
   * 模拟删除变量
   * @param {number[]} indices
   * @returns {number}
   */
  deleteVariables(indices) {
    const namesToDelete = indices
      .map(i => this.a2lVariables[i]?.name)
      .filter(Boolean);

    this.a2lVariables = this.a2lVariables.filter(v => !namesToDelete.includes(v.name));
    this.a2lNames = new Set(this.a2lVariables.map(v => v.name));

    return namesToDelete.length;
  }

  /**
   * 模拟保存 A2L 更改
   * @param {Object[]} edits
   * @returns {SaveResult}
   */
  saveA2lChanges(edits) {
    const result = { modified: 0, deleted: 0, added: 0, skipped: 0 };

    for (const edit of edits) {
      const action = edit.action;
      const originalName = edit.original_name;

      if (action === 'delete') {
        const idx = this.a2lVariables.findIndex(v => v.name === originalName);
        if (idx >= 0) {
          this.a2lVariables.splice(idx, 1);
          result.deleted++;
        }
      } else if (action === 'modify') {
        const variable = this.a2lVariables.find(v => v.name === originalName);
        if (variable) {
          if (edit.name) variable.name = edit.name;
          if (edit.address) variable.address = edit.address;
          if (edit.data_type) variable.data_type = edit.data_type;
          if (edit.var_type) variable.var_type = edit.var_type;
          result.modified++;
        }
      } else if (action === 'add') {
        if (edit.entry && !this.a2lNames.has(edit.original_name)) {
          const entry = edit.entry;
          this.a2lVariables.push({
            name: entry.full_name,
            address: `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}`,
            data_type: 'UBYTE',
            var_type: edit.export_mode === 'characteristic' ? 'CHARACTERISTIC' : 'MEASUREMENT',
          });
          this.a2lNames.add(entry.full_name);
          result.added++;
        } else {
          result.skipped++;
        }
      }
    }

    this.a2lNames = new Set(this.a2lVariables.map(v => v.name));
    return result;
  }

  /**
   * 设置字节序
   * @param {string} endianness
   */
  setEndianness(endianness) {
    if (endianness !== 'little' && endianness !== 'big') {
      throw new Error("无效的字节序，必须是 'little' 或 'big'");
    }
    this.endianness = endianness;
  }
}

// ============================================================
// Tauri API Mock 实现
// ============================================================

// 创建全局状态实例
const mockState = new MockAppState();

/**
 * 模拟 Tauri invoke 函数
 * @param {string} cmd - 命令名称
 * @param {Object} args - 命令参数
 * @returns {Promise<any>}
 */
async function mockInvoke(cmd, args = {}) {
  // 模拟网络延迟
  await new Promise(resolve => setTimeout(resolve, 50 + Math.random() * 100));

  switch (cmd) {
    case 'load_elf':
      return mockState.loadElf(args.path);

    case 'load_package':
      return mockState.loadPackage(args.path);

    case 'generate_package':
      return mockState.generatePackage(args.elfPath, args.outputPath);

    case 'load_a2l':
      return mockState.loadA2l(args.path);

    case 'search_elf_entries':
      return mockState.searchElfEntries(
        args.query,
        args.offset,
        args.limit,
        args.sortField,
        args.sortOrder
      );

    case 'get_elf_count':
      return mockState.getElfCount();

    case 'search_a2l_variables':
      return mockState.searchA2lVariables(args.query, args.offset, args.limit);

    case 'export_entries':
      return mockState.exportEntries(args.indices, args.mode);

    case 'delete_variables':
      return mockState.deleteVariables(args.indices);

    case 'save_a2l_changes':
      return mockState.saveA2lChanges(args.edits);

    case 'set_endianness':
      return mockState.setEndianness(args.endianness);

    default:
      throw new Error(`Unknown command: ${cmd}`);
  }
}

// ============================================================
// Tauri Plugin Mocks
// ============================================================

/**
 * 模拟 @tauri-apps/plugin-dialog
 */
const mockDialog = {
  /**
   * 模拟打开文件对话框
   * @param {Object} options
   * @returns {Promise<string|null>}
   */
  open: async (options = {}) => {
    // 测试时可以通过 window.__mock_dialog_result__ 设置返回值
    if (typeof window !== 'undefined' && window.__mock_dialog_result__ !== undefined) {
      return window.__mock_dialog_result__;
    }

    // 默认返回模拟路径
    if (options.filters?.[0]?.extensions?.includes('a2ldata')) {
      return '/mock/path/test.elf.a2ldata';
    }
    if (options.filters?.[0]?.extensions?.includes('a2l')) {
      return '/mock/path/test.a2l';
    }
    if (options.filters?.[0]?.extensions?.some(e => ['elf', 'out', 'axf'].includes(e))) {
      return '/mock/path/test.elf';
    }
    return '/mock/path/test.file';
  },

  /**
   * 模拟保存文件对话框
   * @param {Object} options
   * @returns {Promise<string|null>}
   */
  save: async (options = {}) => {
    if (typeof window !== 'undefined' && window.__mock_save_result__ !== undefined) {
      return window.__mock_save_result__;
    }
    return options.defaultPath || '/mock/path/save.file';
  },
};

/**
 * 模拟 @tauri-apps/plugin-clipboard-manager
 */
const mockClipboard = {
  /**
   * 写入文本到剪贴板
   * @param {string} text
   * @returns {Promise<void>}
   */
  writeText: async (text) => {
    if (typeof window !== 'undefined') {
      // 实际写入浏览器剪贴板
      await navigator.clipboard.writeText(text);
    }
    // 存储到 mock 状态供测试验证
    mockState._lastClipboardText = text;
  },

  /**
   * 从剪贴板读取文本
   * @returns {Promise<string>}
   */
  readText: async () => {
    if (typeof window !== 'undefined') {
      return navigator.clipboard.readText();
    }
    return mockState._lastClipboardText || '';
  },
};

/**
 * 模拟 @tauri-apps/api/window
 */
const mockWindow = {
  getCurrentWindow: () => ({
    listen: (event, callback) => {
      // 存储监听器供测试触发
      if (!mockState._eventListeners) {
        mockState._eventListeners = {};
      }
      mockState._eventListeners[event] = callback;

      return Promise.resolve(() => {
        delete mockState._eventListeners[event];
      });
    },
    destroy: async () => {
      // 模拟窗口关闭
    },
  }),
};

/**
 * 模拟 @tauri-apps/api/event
 */
const mockEvent = {
  listen: (event, callback) => {
    if (!mockState._eventListeners) {
      mockState._eventListeners = {};
    }
    mockState._eventListeners[event] = callback;

    return Promise.resolve(() => {
      delete mockState._eventListeners[event];
    });
  },
};

// ============================================================
// 测试辅助函数
// ============================================================

/**
 * 重置 mock 状态
 */
function resetMockState() {
  mockState.elfEntries = generateMockElfEntries(1000);
  mockState.a2lVariables = generateMockA2lVariables(500, mockState.elfEntries);
  mockState.elfPath = null;
  mockState.a2lPath = null;
  mockState.a2lNames = new Set();
  mockState.endianness = 'little';
  mockState.packageMeta = null;
  mockState._lastClipboardText = undefined;
  mockState._eventListeners = {};
}

/**
 * 设置 mock ELF 条目
 * @param {A2lEntry[]} entries
 */
function setMockElfEntries(entries) {
  mockState.elfEntries = entries;
}

/**
 * 设置 mock A2L 变量
 * @param {A2lVariable[]} variables
 */
function setMockA2lVariables(variables) {
  mockState.a2lVariables = variables;
  mockState.a2lNames = new Set(variables.map(v => v.name));
}

/**
 * 触发 mock 事件
 * @param {string} event
 * @param {any} payload
 */
function triggerMockEvent(event, payload) {
  if (mockState._eventListeners?.[event]) {
    mockState._eventListeners[event]({ payload });
  }
}

/**
 * 获取 mock 状态(用于断言)
 * @returns {MockAppState}
 */
function getMockState() {
  return mockState;
}

// ============================================================
// 浏览器环境注入
// ============================================================

/**
 * 在浏览器环境中注入 mock API
 */
function injectMockApi() {
  if (typeof window === 'undefined') return;

  // 创建 mock Tauri 模块
  window.__TAURI__ = {
    invoke: mockInvoke,
  };

  // 设置 window 全局 mock
  window.__mock_invoke__ = mockInvoke;
  window.__mock_dialog__ = mockDialog;
  window.__mock_clipboard__ = mockClipboard;
  window.__mock_window__ = mockWindow;
  window.__mock_event__ = mockEvent;

  // 测试辅助函数
  window.__test_helpers__ = {
    resetMockState,
    setMockElfEntries,
    setMockA2lVariables,
    triggerMockEvent,
    getMockState,
    generateMockElfEntries,
    generateMockA2lVariables,
  };

  console.log('[Mock API] Tauri mock API injected');
}

// 自动注入
injectMockApi();

// ============================================================
// 导出 (用于 Node.js 测试环境)
// ============================================================

if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    mockInvoke,
    mockDialog,
    mockClipboard,
    mockWindow,
    mockEvent,
    MockAppState,
    generateMockElfEntries,
    generateMockA2lVariables,
    resetMockState,
    setMockElfEntries,
    setMockA2lVariables,
    triggerMockEvent,
    getMockState,
  };
}
