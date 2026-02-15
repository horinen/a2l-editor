import { writable, derived, get } from 'svelte/store';
import type { A2lEntry, A2lVariable, ThemeName } from './types';

// 排序类型定义
export type SortField = 'name' | 'address';
export type SortOrder = 'asc' | 'desc';
export interface SortConfig {
  field: SortField;
  order: SortOrder;
}

// ELF 变量 (右侧面板)
export const elfEntries = writable<A2lEntry[]>([]);
export const elfFilteredCount = writable<number>(0);
export const elfTotalCount = writable<number>(0);
export const elfSearchQuery = writable<string>('');
export const elfSelectedIndices = writable<Set<number>>(new Set());
export const lastElfSelectedDisplayPos = writable<number | null>(null);
export const elfSortConfigs = writable<SortConfig[]>([{ field: 'name', order: 'asc' }]);

// A2L 变量 (左侧面板)
export const a2lVariables = writable<A2lVariable[]>([]);
export const a2lSearchQuery = writable<string>('');
export const a2lSelectedIndices = writable<Set<number>>(new Set());
export const lastA2lSelectedIndex = writable<number | null>(null);
export const a2lSortConfigs = writable<SortConfig[]>([{ field: 'name', order: 'asc' }]);

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
export const exportMode = writable<'measurement' | 'characteristic'>('measurement');
export const exportPreview = writable<{ added: number; skipped: number; existing: number } | null>(null);

// 主题
export const currentTheme = writable<ThemeName>('dark');

// 派生状态
export const elfSelectedCount = derived(elfSelectedIndices, $set => $set.size);
export const a2lSelectedCount = derived(a2lSelectedIndices, $set => $set.size);

// 清除 ELF 选择
export function clearElfSelection() {
  elfSelectedIndices.set(new Set());
}

// 清除 A2L 选择
export function clearA2lSelection() {
  a2lSelectedIndices.set(new Set());
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
export function toggleA2lSelection(index: number, ctrlKey: boolean, shiftKey: boolean, totalCount?: number) {
  a2lSelectedIndices.update(set => {
    const newSet = new Set(set);
    
    if (shiftKey && totalCount !== undefined && totalCount > 0) {
      const lastIndex = get(lastA2lSelectedIndex);
      if (lastIndex !== null) {
        const start = Math.min(lastIndex, index);
        const end = Math.max(lastIndex, index);
        for (let i = start; i <= end; i++) {
          newSet.add(i);
        }
        return newSet;
      }
    }
    
    if (ctrlKey) {
      if (newSet.has(index)) {
        newSet.delete(index);
      } else {
        newSet.add(index);
      }
      lastA2lSelectedIndex.set(index);
    } else {
      newSet.clear();
      newSet.add(index);
      lastA2lSelectedIndex.set(index);
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
    const indices = new Set(vars.map((_, i) => i));
    a2lSelectedIndices.set(indices);
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
  getFieldValue: (item: T, field: SortField) => string | number
): T[] {
  if (configs.length === 0) return items;
  
  return [...items].sort((a, b) => {
    for (const config of configs) {
      const valueA = getFieldValue(a, config.field);
      const valueB = getFieldValue(b, config.field);
      
      let comparison = 0;
      if (typeof valueA === 'string' && typeof valueB === 'string') {
        comparison = valueA.localeCompare(valueB);
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
