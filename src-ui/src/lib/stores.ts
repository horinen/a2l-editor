import { writable, derived, get } from 'svelte/store';
import type { A2lEntry, A2lVariable, ThemeName } from './types';

// 排序类型定义
export type SortField = 'name' | 'address';
export type SortOrder = 'asc' | 'desc';
export interface SortConfig {
  field: SortField;
  order: SortOrder;
}

export type SortMode = 'lexicographic' | 'natural';

export function matchQuery(name: string, query: string): boolean {
  const nameLower = name.toLowerCase();
  let q = query;
  let starts = false;
  let ends = false;

  if (q.startsWith('^')) {
    starts = true;
    q = q.slice(1);
  }
  if (q.endsWith('$')) {
    ends = true;
    q = q.slice(0, -1);
  }

  const qLower = q.toLowerCase();

  if (starts && ends) return nameLower === qLower;
  if (starts) return nameLower.startsWith(qLower);
  if (ends) return nameLower.endsWith(qLower);
  return nameLower.includes(qLower);
}

export function naturalCompare(a: string, b: string): number {
  let ia = 0;
  let ib = 0;

  while (ia < a.length && ib < b.length) {
    const ca = a[ia];
    const cb = b[ib];

    if (ca >= '0' && ca <= '9' && cb >= '0' && cb <= '9') {
      let na = 0;
      while (ia < a.length && a[ia] >= '0' && a[ia] <= '9') {
        na = na * 10 + (a.charCodeAt(ia) - 48);
        ia++;
      }
      let nb = 0;
      while (ib < b.length && b[ib] >= '0' && b[ib] <= '9') {
        nb = nb * 10 + (b.charCodeAt(ib) - 48);
        ib++;
      }
      if (na !== nb) return na - nb;
    } else {
      if (ca !== cb) return ca < cb ? -1 : 1;
      ia++;
      ib++;
    }
  }

  return (a.length - ia) - (b.length - ib);
}

// ELF 变量 (右侧面板)
export const elfEntries = writable<A2lEntry[]>([]);
export const elfFilteredCount = writable<number>(0);
export const elfTotalCount = writable<number>(0);
export const elfSearchQuery = writable<string>('');
export const elfSelectedIndices = writable<Set<number>>(new Set());
export const lastElfSelectedDisplayPos = writable<number | null>(null);
export const elfSortConfigs = writable<SortConfig[]>([{ field: 'name', order: 'asc' }]);
export const elfSortMode = writable<SortMode>('lexicographic');

// A2L 变量 (左侧面板)
export const a2lVariables = writable<A2lVariable[]>([]);
export const a2lSearchQuery = writable<string>('');
export const a2lSelectedNames = writable<Set<string>>(new Set());
export const lastA2lSelectedName = writable<string | null>(null);
export const a2lSortConfigs = writable<SortConfig[]>([{ field: 'name', order: 'asc' }]);
export const a2lSortMode = writable<SortMode>('lexicographic');

// 文件状态
export const elfPath = writable<string | null>(null);
export const elfFileName = writable<string | null>(null);
export const elfFileSize = writable<string | null>(null);
export const packagePath = writable<string | null>(null);
export const a2lPath = writable<string | null>(null);
export const a2lNames = writable<Set<string>>(new Set());

// 应用状态
export const statusMessage = writable<string>('💡 文件 → 打开 ELF 开始使用');
export const isLoading = writable<boolean>(false);
export const loadProgress = writable<number>(0);

// 对话框状态
export const showExportDialog = writable<boolean>(false);
export const showGenerateDialog = writable<boolean>(false);
export const showAboutDialog = writable<boolean>(false);
export const showHelpDialog = writable<boolean>(false);
export const showCompuMethodPanel = writable<boolean>(false);
export const exportMode = writable<'measurement' | 'characteristic'>('measurement');
export const exportPreview = writable<{ added: number; skipped: number; existing: number } | null>(null);

// 主题
export const currentTheme = writable<ThemeName>('dark');

export const endianness = writable<'little' | 'big'>('little');

// 派生状态
export const elfSelectedCount = derived(elfSelectedIndices, $set => $set.size);
export const a2lSelectedCount = derived(a2lSelectedNames, $set => $set.size);

// 清除 ELF 选择
export function clearElfSelection() {
  elfSelectedIndices.set(new Set());
}

// 清除 A2L 选择
export function clearA2lSelection() {
  a2lSelectedNames.set(new Set());
}

// 切换 ELF 选中
// displayIndex: 当前点击项在显示列表中的位置（0, 1, 2, ...）
// entryIndex: 当前点击项的原始索引（entry.index）
// displayIndices: 显示列表中所有项的原始索引数组
export function toggleElfSelection(
  displayIndex: number, 
  entryIndex: number, 
  ctrlKey: boolean, 
  shiftKey: boolean, 
  displayIndices: number[]
) {
  elfSelectedIndices.update(set => {
    const newSet = new Set(set);
    
    if (shiftKey && displayIndices.length > 0) {
      const lastPos = get(lastElfSelectedDisplayPos);
      if (lastPos !== null) {
        const start = Math.min(lastPos, displayIndex);
        const end = Math.max(lastPos, displayIndex);
        for (let i = start; i <= end; i++) {
          newSet.add(displayIndices[i]);
        }
        return newSet;
      }
    }
    
    if (ctrlKey) {
      if (newSet.has(entryIndex)) {
        newSet.delete(entryIndex);
      } else {
        newSet.add(entryIndex);
      }
      lastElfSelectedDisplayPos.set(displayIndex);
    } else {
      newSet.clear();
      newSet.add(entryIndex);
      lastElfSelectedDisplayPos.set(displayIndex);
    }
    return newSet;
  });
}

// 切换 A2L 选中
// name: 当前点击项的变量名
// allNames: 显示列表中所有项的名称数组（用于 Shift 多选）
export function toggleA2lSelection(
  name: string, 
  ctrlKey: boolean, 
  shiftKey: boolean, 
  allNames: string[]
) {
  a2lSelectedNames.update(set => {
    const newSet = new Set(set);
    
    if (shiftKey && allNames.length > 0) {
      const lastName = get(lastA2lSelectedName);
      if (lastName !== null) {
        const startIdx = allNames.indexOf(lastName);
        const endIdx = allNames.indexOf(name);
        if (startIdx !== -1 && endIdx !== -1) {
          const start = Math.min(startIdx, endIdx);
          const end = Math.max(startIdx, endIdx);
          for (let i = start; i <= end; i++) {
            newSet.add(allNames[i]);
          }
          return newSet;
        }
      }
    }
    
    if (ctrlKey) {
      if (newSet.has(name)) {
        newSet.delete(name);
      } else {
        newSet.add(name);
      }
      lastA2lSelectedName.set(name);
    } else {
      newSet.clear();
      newSet.add(name);
      lastA2lSelectedName.set(name);
    }
    return newSet;
  });
}

// 全选 ELF
export function selectAllElf() {
  elfEntries.update(entries => {
    const indices = new Set(entries.map((_, i) => i));
    elfSelectedIndices.set(indices);
    return entries;
  });
}

// 全选 A2L
export function selectAllA2l() {
  a2lVariables.update(vars => {
    const names = new Set(vars.map(v => v.name));
    a2lSelectedNames.set(names);
    return vars;
  });
}

// 排序工具函数
export function toggleSort(configs: SortConfig[], field: SortField, shiftKey: boolean): SortConfig[] {
  const existingIndex = configs.findIndex(c => c.field === field);
  
  if (existingIndex === -1) {
    // 新字段
    if (shiftKey) {
      return [...configs, { field, order: 'asc' }];
    } else {
      return [{ field, order: 'asc' }];
    }
  }
  
  const existing = configs[existingIndex];
  if (existing.order === 'asc') {
    // 升序变降序
    const newConfigs = [...configs];
    newConfigs[existingIndex] = { field, order: 'desc' };
    return newConfigs;
  } else {
    // 降序则移除该排序
    const newConfigs = configs.filter(c => c.field !== field);
    return newConfigs.length > 0 ? newConfigs : [{ field: 'name', order: 'asc' }];
  }
}

export function applySorting<T>(
  items: T[], 
  configs: SortConfig[], 
  getFieldValue: (item: T, field: SortField) => string | number,
  sortMode: SortMode = 'lexicographic'
): T[] {
  if (configs.length === 0) return items;
  
  return [...items].sort((a, b) => {
    for (const config of configs) {
      const valueA = getFieldValue(a, config.field);
      const valueB = getFieldValue(b, config.field);
      
      let comparison = 0;
      if (typeof valueA === 'string' && typeof valueB === 'string') {
        if (sortMode === 'natural') {
          comparison = naturalCompare(valueA, valueB);
        } else {
          comparison = valueA.localeCompare(valueB);
        }
      } else {
        comparison = (valueA as number) - (valueB as number);
      }
      
      if (comparison !== 0) {
        return config.order === 'asc' ? comparison : -comparison;
      }
    }
    return 0;
  });
}

export function parseAddress(addr: string | null): number {
  if (!addr) return 0;
  const hex = addr.replace(/^0x/i, '');
  return parseInt(hex, 16) || 0;
}
