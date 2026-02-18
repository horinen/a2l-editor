// 自包含的 Tauri Core API Mock
// 模拟 a2l-editor 的所有 Tauri 命令
// 版本: 2.0 - 支持选中状态修复和 bitfield

// ========== Mock 数据 ==========
const mockElfEntries = [
  { index: 0, full_name: 'app_variable_0001', address: 0x20000000, size: 4, a2l_type: 'ULONG', type_name: 'uint32', bit_offset: null, bit_size: null },
  { index: 1, full_name: 'app_variable_0002', address: 0x20000004, size: 2, a2l_type: 'UWORD', type_name: 'uint16', bit_offset: null, bit_size: null },
  { index: 2, full_name: 'sys_variable_0003', address: 0x20000008, size: 1, a2l_type: 'UBYTE', type_name: 'uint8', bit_offset: null, bit_size: null },
  { index: 3, full_name: 'drv_variable_0004', address: 0x20000010, size: 4, a2l_type: 'FLOAT32_IEEE', type_name: 'float', bit_offset: null, bit_size: null },
  { index: 4, full_name: 'cal_variable_0005', address: 0x20000020, size: 8, a2l_type: 'FLOAT64_IEEE', type_name: 'double', bit_offset: null, bit_size: null },
  // bitfield 变量
  { index: 5, full_name: 'bitfield_status_flags', address: 0x20000030, size: 4, a2l_type: 'UBYTE', type_name: 'uint8_t', bit_offset: 0, bit_size: 3 },
  { index: 6, full_name: 'bitfield_mode_bits', address: 0x20000030, size: 4, a2l_type: 'UBYTE', type_name: 'uint8_t', bit_offset: 3, bit_size: 2 },
  { index: 7, full_name: 'bitfield_counter', address: 0x20000034, size: 4, a2l_type: 'UWORD', type_name: 'uint16_t', bit_offset: 0, bit_size: 8 },
  { index: 8, full_name: 'test_bitfield_calc', address: 0x20000040, size: 4, a2l_type: 'UBYTE', type_name: 'uint8_t', bit_offset: 4, bit_size: 3 },
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
let lastGeneratedBlock = '';
let deletedVariableNames = [];
let clipboardContent = '';

// ========== BIT_MASK 计算 ==========
function calculateBitMask(bitOffset, bitSize) {
  if (bitOffset === null || bitSize === null) return 0;
  return ((1 << bitSize) - 1) << bitOffset;
}

// ========== A2L 块生成 ==========
function generateMeasurementBlock(entry) {
  const a2lType = entry.a2l_type;
  const formatMap = {
    'UBYTE': '%3.0', 'SBYTE': '%3.0',
    'UWORD': '%5.0', 'SWORD': '%5.0',
    'ULONG': '%10.0', 'SLONG': '%10.0',
    'A_UINT64': '%20.0', 'A_INT64': '%20.0',
    'FLOAT32_IEEE': '%10.4', 'FLOAT64_IEEE': '%16.8'
  };
  const minMaxMap = {
    'UBYTE': ['0', '255'], 'SBYTE': ['-128', '127'],
    'UWORD': ['0', '65535'], 'SWORD': ['-32768', '32767'],
    'ULONG': ['0', '4294967295'], 'SLONG': ['-2147483648', '2147483647'],
    'A_UINT64': ['0', '18446744073709551615'], 'A_INT64': ['-9223372036854775808', '9223372036854775807'],
    'FLOAT32_IEEE': ['-3.4E38', '3.4E38'], 'FLOAT64_IEEE': ['-1.7E308', '1.7E308']
  };

  const isBitfield = entry.bit_size !== null && entry.bit_size !== undefined;
  let minVal, maxVal;
  
  if (isBitfield) {
    minVal = '0';
    maxVal = String((1 << entry.bit_size) - 1);
  } else {
    [minVal, maxVal] = minMaxMap[a2lType] || ['0', '0'];
  }

  let block = `    /begin MEASUREMENT ${entry.full_name} ""\n`;
  block += `      ${a2lType} NO_COMPU_METHOD 0 0 ${minVal} ${maxVal}\n`;
  
  if (isBitfield) {
    const mask = calculateBitMask(entry.bit_offset, entry.bit_size);
    block += `      BIT_MASK 0x${mask.toString(16).toUpperCase()}\n`;
  }
  
  block += `      ECU_ADDRESS 0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}\n`;
  block += `      ECU_ADDRESS_EXTENSION 0x0\n`;
  block += `      FORMAT "${formatMap[a2lType] || '%10.0'}"\n`;
  block += `      SYMBOL_LINK "${entry.full_name}" 0\n`;
  block += `    /end MEASUREMENT\n\n`;

  lastGeneratedBlock = block;
  return block;
}

function generateCharacteristicBlock(entry) {
  const a2lType = entry.a2l_type;
  const recordLayoutMap = {
    'UBYTE': '__UByte_Value', 'SBYTE': '__SByte_Value',
    'UWORD': '__UWord_Value', 'SWORD': '__SWord_Value',
    'ULONG': '__ULong_Value', 'SLONG': '__SLong_Value',
    'A_UINT64': '__UInt64_Value', 'A_INT64': '__Int64_Value',
    'FLOAT32_IEEE': '__Float32_Value', 'FLOAT64_IEEE': '__Float64_Value'
  };

  const isBitfield = entry.bit_size !== null && entry.bit_size !== undefined;
  let maxVal;
  
  if (isBitfield) {
    maxVal = String((1 << entry.bit_size) - 1);
  } else {
    const minMaxMap = {
      'UBYTE': ['0', '255'], 'SBYTE': ['-128', '127'],
      'UWORD': ['0', '65535'], 'SWORD': ['-32768', '32767'],
      'ULONG': ['0', '4294967295'], 'SLONG': ['-2147483648', '2147483647'],
      'A_UINT64': ['0', '18446744073709551615'], 'A_INT64': ['-9223372036854775808', '9223372036854775807'],
      'FLOAT32_IEEE': ['-3.4E38', '3.4E38'], 'FLOAT64_IEEE': ['-1.7E308', '1.7E308']
    };
    [, maxVal] = minMaxMap[a2lType] || ['0', '0'];
  }

  let block = `    /begin CHARACTERISTIC ${entry.full_name} ""\n`;
  block += `      VALUE 0x${entry.address.toString(16).toUpperCase().padStart(8, '0')} ${recordLayoutMap[a2lType] || '__ULong_Value'} 0 NO_COMPU_METHOD 0 ${maxVal}\n`;
  
  if (isBitfield) {
    const mask = calculateBitMask(entry.bit_offset, entry.bit_size);
    block += `      BIT_MASK 0x${mask.toString(16).toUpperCase()}\n`;
  }
  
  block += `      EXTENDED_LIMITS 0 ${maxVal}\n`;
  block += `      SYMBOL_LINK "${entry.full_name}" 0\n`;
  block += `    /end CHARACTERISTIC\n\n`;

  lastGeneratedBlock = block;
  return block;
}

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

      // 排序
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
      // 使用索引获取条目并生成 A2L 块
      const indicesToExport = args.indices || [];
      const entriesToExport = indicesToExport.map(i => elfEntriesStore[i]).filter(Boolean);
      
      const kind = args.mode === 'characteristic' ? 'characteristic' : 'measurement';
      let addedCount = 0;
      
      for (const entry of entriesToExport) {
        if (!a2lVariablesStore.find(v => v.name === entry.full_name)) {
          // 生成 A2L 块
          if (kind === 'characteristic') {
            generateCharacteristicBlock(entry);
          } else {
            generateMeasurementBlock(entry);
          }
          
          // 添加到 A2L 变量列表
          a2lVariablesStore.push({
            name: entry.full_name,
            address: `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}`,
            data_type: entry.a2l_type,
            var_type: kind === 'characteristic' ? 'CHARACTERISTIC' : 'MEASUREMENT'
          });
          addedCount++;
        }
      }
      
      return { added: addedCount, skipped: indicesToExport.length - addedCount, existing: a2lVariablesStore.length - addedCount };

    case 'delete_variables':
      // 使用变量名删除 (修复 #8)
      const namesToDelete = args.names || [];
      deletedVariableNames = [...namesToDelete];
      
      const beforeCount = a2lVariablesStore.length;
      a2lVariablesStore = a2lVariablesStore.filter(v => !namesToDelete.includes(v.name));
      const deletedCount = beforeCount - a2lVariablesStore.length;
      
      return deletedCount;

    case 'save_a2l_changes':
      const edits = args.edits || [];
      let result = { modified: 0, deleted: 0, added: 0, skipped: 0 };
      
      for (const edit of edits) {
        if (edit.action === 'modify') {
          const variable = a2lVariablesStore.find(v => v.name === edit.original_name);
          if (variable) {
            if (edit.name) variable.name = edit.name;
            if (edit.address) variable.address = edit.address;
            if (edit.data_type) variable.data_type = edit.data_type;
            if (edit.var_type) variable.var_type = edit.var_type;
            result.modified++;
          }
        } else if (edit.action === 'delete') {
          const beforeCount = a2lVariablesStore.length;
          a2lVariablesStore = a2lVariablesStore.filter(v => v.name !== edit.original_name);
          if (a2lVariablesStore.length < beforeCount) result.deleted++;
        } else if (edit.action === 'add' && edit.entry) {
          if (!a2lVariablesStore.find(v => v.name === edit.entry.full_name)) {
            const entry = edit.entry;
            const kind = edit.export_mode === 'characteristic' ? 'characteristic' : 'measurement';
            
            // 生成 A2L 块
            if (kind === 'characteristic') {
              generateCharacteristicBlock(entry);
            } else {
              generateMeasurementBlock(entry);
            }
            
            a2lVariablesStore.push({
              name: entry.full_name,
              address: `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}`,
              data_type: entry.a2l_type,
              var_type: kind === 'characteristic' ? 'CHARACTERISTIC' : 'MEASUREMENT'
            });
            result.added++;
          } else {
            result.skipped++;
          }
        }
      }
      
      return result;

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
    packageMeta,
    lastGeneratedBlock,
    deletedVariableNames,
    clipboardContent
  };
}

export function setMockElfEntries(entries) {
  elfEntriesStore = entries.map((e, i) => ({ ...e, index: i }));
}

export function setMockA2lVariables(variables) {
  a2lVariablesStore = [...variables];
}

export function getLastGeneratedBlock() {
  return lastGeneratedBlock;
}

export function getDeletedVariableNames() {
  return [...deletedVariableNames];
}

export function resetMockState() {
  elfEntriesStore = [...mockElfEntries];
  a2lVariablesStore = [...mockA2lVariables];
  isLoaded = false;
  packageMeta = null;
  lastGeneratedBlock = '';
  deletedVariableNames = [];
  clipboardContent = '';
}

export function setClipboardContent(text) {
  clipboardContent = text;
}

export function getClipboardContent() {
  return clipboardContent;
}

export default {
  invoke,
  getMockState,
  setMockElfEntries,
  setMockA2lVariables,
  getLastGeneratedBlock,
  getDeletedVariableNames,
  resetMockState,
  setClipboardContent,
  getClipboardContent,
  calculateBitMask,
  generateMeasurementBlock,
  generateCharacteristicBlock
};
