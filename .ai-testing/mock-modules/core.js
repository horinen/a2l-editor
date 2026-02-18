// 自包含的 Tauri Core API Mock
// 模拟 a2l-editor 的所有 Tauri 命令
// 更新版本：返回数据格式与 commands.rs 保持一致

// ========== Mock 数据 ==========
// 数据格式与 commands.rs 中的 EntryInfo/VariableInfo 一致
const mockElfEntries = [
  { index: 0, full_name: 'app_variable_0001', address: 0x20000000, size: 4, a2l_type: 'SCALAR', type_name: 'uint32', bit_offset: null, bit_size: null },
  { index: 1, full_name: 'app_variable_0002', address: 0x20000004, size: 2, a2l_type: 'SCALAR', type_name: 'uint16', bit_offset: null, bit_size: null },
  { index: 2, full_name: 'sys_variable_0003', address: 0x20000008, size: 1, a2l_type: 'SCALAR', type_name: 'uint8', bit_offset: null, bit_size: null },
  { index: 3, full_name: 'drv_variable_0004', address: 0x20000010, size: 4, a2l_type: 'ARRAY', type_name: 'float', bit_offset: null, bit_size: null },
  { index: 4, full_name: 'cal_variable_0005', address: 0x20000020, size: 8, a2l_type: 'SCALAR', type_name: 'double', bit_offset: null, bit_size: null },
  { index: 5, full_name: 'meas_variable_0006', address: 0x20000030, size: 4, a2l_type: 'SCALAR', type_name: 'int32', bit_offset: null, bit_size: null },
  { index: 6, full_name: 'diag_variable_0007', address: 0x20000040, size: 2, a2l_type: 'SCALAR', type_name: 'int16', bit_offset: null, bit_size: null },
  { index: 7, full_name: 'ctrl_variable_0008', address: 0x20000050, size: 1, a2l_type: 'SCALAR', type_name: 'int8', bit_offset: null, bit_size: null },
  { index: 8, full_name: 'sensor_variable_0009', address: 0x20000060, size: 4, a2l_type: 'STRUCT', type_name: 'uint32', bit_offset: null, bit_size: null },
  { index: 9, full_name: 'actuator_variable_0010', address: 0x20000070, size: 2, a2l_type: 'SCALAR', type_name: 'uint16', bit_offset: null, bit_size: null },
];

const mockA2lVariables = [
  { name: 'existing_variable_0001', address: '0x20010000', data_type: 'ULONG', var_type: 'MEASUREMENT' },
  { name: 'existing_variable_0002', address: '0x20010004', data_type: 'UWORD', var_type: 'MEASUREMENT' },
  { name: 'legacy_variable_0003', address: '0x20010008', data_type: 'UBYTE', var_type: 'CHARACTERISTIC' },
];

// ========== Mock 状态 ==========
let elfEntriesStore = [...mockElfEntries];
let a2lVariablesStore = [...mockA2lVariables];
let isLoaded = false;
let packageMeta = null;

// ========== Tauri 命令模拟 ==========
export async function invoke(cmd, args = {}) {
  console.log('[Mock] invoke:', cmd, args);

  // 模拟网络延迟
  await new Promise(resolve => setTimeout(resolve, 30 + Math.random() * 50));

  switch (cmd) {
    case 'load_elf':
    case 'load_package':
      isLoaded = true;
      packageMeta = {
        file_name: args.path?.split('/').pop()?.replace('.a2ldata', '.elf') || 'test.elf',
        elf_path: args.path?.replace('.a2ldata', '.elf') || '/mock/test.elf',
        entry_count: elfEntriesStore.length,
        created_at: Date.now() - 86400000
      };
      return {
        meta: packageMeta,
        entry_count: elfEntriesStore.length
      };

    case 'load_a2l':
      return {
        path: args.path || '/mock/test.a2l',
        variable_count: a2lVariablesStore.length,
        existing_names: a2lVariablesStore.map(v => v.name)
      };

    case 'generate_package':
      isLoaded = true;
      packageMeta = {
        file_name: args.elfPath?.split('/').pop() || 'test.elf',
        elf_path: args.elfPath || '/mock/test.elf',
        entry_count: elfEntriesStore.length,
        created_at: Date.now()
      };
      return packageMeta;

    case 'search_elf_entries':
      const { query, offset = 0, limit = 10000, sortField, sortOrder } = args;
      let results = [...elfEntriesStore];

      // 过滤
      if (query) {
        const q = query.toLowerCase();
        results = results.filter(e =>
          e.full_name.toLowerCase().includes(q)
        );
      }

      // 排序 - 与 commands.rs 逻辑一致
      results.sort((a, b) => {
        const cmp = sortField === 'address'
          ? a.address - b.address
          : a.full_name.localeCompare(b.full_name);
        return sortOrder === 'desc' ? -cmp : cmp;
      });

      return results.slice(offset, offset + limit);

    case 'get_elf_count':
      return elfEntriesStore.length;

    case 'search_a2l_variables':
      const a2lQuery = args.query;
      let a2lResults = [...a2lVariablesStore];
      if (a2lQuery) {
        a2lResults = a2lResults.filter(v =>
          v.name.toLowerCase().includes(a2lQuery.toLowerCase())
        );
      }
      return a2lResults.slice(args.offset || 0, (args.offset || 0) + (args.limit || 10000));

    case 'export_entries':
      const added = args.indices?.length || 0;
      return { added, skipped: 0, existing: 0 };

    case 'delete_variables':
      return args.indices?.length || 0;

    case 'save_a2l_changes':
      return { modified: 0, deleted: 0, added: args.edits?.length || 0, skipped: 0 };

    case 'set_endianness':
      return void 0;

    default:
      console.warn('[Mock] Unknown command:', cmd);
      return null;
  }
}

// ========== 测试辅助函数 ==========
export function getMockState() {
  return {
    elfEntries: elfEntriesStore,
    a2lVariables: a2lVariablesStore,
    isLoaded,
    packageMeta
  };
}

export function setMockElfEntries(entries) {
  elfEntriesStore = entries;
}

export function setMockA2lVariables(variables) {
  a2lVariablesStore = variables;
}

export function resetMockState() {
  elfEntriesStore = [...mockElfEntries];
  a2lVariablesStore = [...mockA2lVariables];
  isLoaded = false;
  packageMeta = null;
}

export default {
  invoke,
  getMockState,
  setMockElfEntries,
  setMockA2lVariables,
  resetMockState
};
