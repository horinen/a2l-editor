export interface RecentFile {
  path: string;
  name: string;
  timestamp: number;
}

const MAX_RECENT = 5;

const ELF_KEY = 'recent-elf-files';
const A2L_KEY = 'recent-a2l-files';

function getFiles(key: string): RecentFile[] {
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveFiles(key: string, files: RecentFile[]) {
  localStorage.setItem(key, JSON.stringify(files));
}

function addFile(key: string, path: string, name: string): RecentFile[] {
  let files = getFiles(key);
  files = files.filter(f => f.path !== path);
  files.unshift({ path, name, timestamp: Date.now() });
  if (files.length > MAX_RECENT) files = files.slice(0, MAX_RECENT);
  saveFiles(key, files);
  return files;
}

function removeFile(key: string, path: string): RecentFile[] {
  let files = getFiles(key).filter(f => f.path !== path);
  saveFiles(key, files);
  return files;
}

function clearFiles(key: string) {
  localStorage.removeItem(key);
}

export function getRecentElfFiles(): RecentFile[] {
  return getFiles(ELF_KEY);
}

export function addRecentElfFile(path: string, name: string): RecentFile[] {
  return addFile(ELF_KEY, path, name);
}

export function removeRecentElfFile(path: string): RecentFile[] {
  return removeFile(ELF_KEY, path);
}

export function clearRecentElfFiles() {
  clearFiles(ELF_KEY);
}

export function getRecentA2lFiles(): RecentFile[] {
  return getFiles(A2L_KEY);
}

export function addRecentA2lFile(path: string, name: string): RecentFile[] {
  return addFile(A2L_KEY, path, name);
}

export function removeRecentA2lFile(path: string): RecentFile[] {
  return removeFile(A2L_KEY, path);
}

export function clearRecentA2lFiles() {
  clearFiles(A2L_KEY);
}

export function getLastElfDir(): string | undefined {
  const files = getFiles(ELF_KEY);
  return files.length > 0 ? files[0].path : undefined;
}

export function getLastA2lDir(): string | undefined {
  const files = getFiles(A2L_KEY);
  return files.length > 0 ? files[0].path : undefined;
}
