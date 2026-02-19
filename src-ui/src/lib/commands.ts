import { invoke } from '@tauri-apps/api/core';
import type { 
  A2lEntry, 
  A2lVariable, 
  A2lVariableEdit,
  LoadResult, 
  PackageMeta, 
  A2lLoadResult, 
  ExportResult,
  SaveResult,
  ExportMode 
} from './types';

// 文件操作
export async function loadElf(path: string): Promise<LoadResult> {
  return invoke('load_elf', { path });
}

export async function loadPackage(path: string): Promise<LoadResult> {
  return invoke('load_package', { path });
}

export async function generatePackage(elfPath: string, outputPath?: string): Promise<PackageMeta> {
  return invoke('generate_package', { elfPath, outputPath });
}

export async function loadA2l(path: string): Promise<A2lLoadResult> {
  return invoke('load_a2l', { path });
}

// 变量查询
export async function searchElfEntries(
  query: string, 
  offset = 0, 
  limit = 10000,
  sortField: 'name' | 'address' = 'name',
  sortOrder: 'asc' | 'desc' = 'asc'
): Promise<A2lEntry[]> {
  return invoke('search_elf_entries', { 
    query, 
    offset, 
    limit,
    sortField,
    sortOrder
  });
}

export async function getElfCount(): Promise<number> {
  return invoke('get_elf_count');
}

export async function searchA2lVariables(query: string, offset = 0, limit = 10000): Promise<A2lVariable[]> {
  return invoke('search_a2l_variables', { query, offset, limit });
}

// 导出/删除
export async function exportEntries(indices: number[], mode: ExportMode): Promise<ExportResult> {
  return invoke('export_entries', { indices, mode });
}

export async function deleteVariables(names: string[]): Promise<number> {
  return invoke('delete_variables', { names });
}

export async function saveA2lChanges(edits: A2lVariableEdit[]): Promise<SaveResult> {
  const inputEdits = edits.map(e => ({
    action: e.action,
    original_name: e.originalName,
    name: e.name,
    address: e.address,
    data_type: e.data_type,
    var_type: e.var_type,
    bit_mask: e.bit_mask,
    entry: e.entry,
    export_mode: e.exportMode,
  }));
  return invoke('save_a2l_changes', { edits: inputEdits });
}

export async function setEndianness(endianness: 'little' | 'big'): Promise<void> {
  return invoke('set_endianness', { endianness });
}
