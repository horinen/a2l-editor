export interface A2lEntry {
  index: number;
  full_name: string;
  address: number;
  size: number;
  a2l_type: string;
  type_name: string;
  bit_offset: number | null;
  bit_size: number | null;
}

export interface A2lVariable {
  name: string;
  address: string | null;
  data_type: string;
  var_type: 'MEASUREMENT' | 'CHARACTERISTIC';
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
