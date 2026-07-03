export interface A2lEntry {
  index: number;
  full_name: string;
  address: number;
  size: number;
  a2l_type: string;
  type_name: string;
  bit_offset: number | null;
  bit_size: number | null;
  symbol_link?: string | null;
}

export interface A2lVariable {
  name: string;
  address: string | null;
  data_type: string;
  var_type: 'MEASUREMENT' | 'CHARACTERISTIC';
  bit_mask: string | null;
  compu_method: string | null;
  symbol_link: string | null;
  f: number | null;
  offset: number | null;
  unit: string | null;
}

export interface LoadResult {
  meta: PackageMeta;
  entry_count: number;
}

export interface PackageMeta {
  file_name: string;
  elf_path: string | null;
  entry_count: number;
  created_at: number;
}

export interface A2lLoadResult {
  path: string;
  variable_count: number;
  existing_names: string[];
}

export interface ExportResult {
  added: number;
  skipped: number;
  existing: number;
}

export type ExportMode = 'measurement' | 'characteristic';
export type ThemeName = 'dark' | 'light' | 'midnight' | 'ocean';

export type EditActionType = 'modify' | 'delete' | 'add';

export interface A2lVariableEdit {
  action: EditActionType;
  originalName: string;
  name?: string;
  address?: string;
  data_type?: string;
  var_type?: 'MEASUREMENT' | 'CHARACTERISTIC';
  bit_mask?: string;
  compu_method?: string;
  f?: number;
  offset?: number;
  unit?: string;
  symbol_link?: string;
  entry?: A2lEntry;
  exportMode?: ExportMode;
}

export interface SaveResult {
  modified: number;
  deleted: number;
  added: number;
  skipped: number;
}

export interface UpdateAddressResult {
  updated: number;
  skipped: number;
}

export interface ExcelImportRow {
  名称: string;
  link: string;
  变量类型: '观测' | '标定';
  转换关系?: string;
}

export interface ExcelImportResult {
  imported: number;
  skipped: number;
  notFound: string[];
}

export type CompuMethodType = 'LINEAR' | 'TAB_VERB' | 'TAB_INTP' | 'IDENTICAL';

export interface TabVerbPair {
  in_val: number;
  verbal: string;
}

export interface TabIntpPair {
  in_val: number;
  out_val: number;
}

export interface CompuMethodDetail {
  name: string;
  conversion_type: CompuMethodType;
  unit: string;
  description: string;
  f: number;
  offset: number;
  verb_pairs: TabVerbPair[];
  default_value: string;
  intp_pairs: TabIntpPair[];
}

export interface CompuMethodSummary {
  name: string;
  conversion_type: CompuMethodType;
  summary: string;
  unit: string;
  ref_count: number;
}

export interface CompuMethodInput {
  name: string;
  conversion_type: CompuMethodType;
  unit: string;
  description: string;
  f: number;
  offset: number;
  verb_pairs: TabVerbPair[];
  default_value: string;
  intp_pairs: TabIntpPair[];
}

export interface PreviewResult {
  raw: number;
  physical: number | null;
  verbal: string | null;
}
