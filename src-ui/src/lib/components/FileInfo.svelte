<script lang="ts">
  import { elfPath, elfFileName, elfTotalCount, packagePath, a2lPath, a2lVariables, isLoading, showGenerateDialog } from '$lib/stores';
  import { open } from '@tauri-apps/plugin-dialog';
  import { loadElf, loadPackage, loadA2l, searchElfEntries, searchA2lVariables } from '$lib/commands';
  import { elfEntries, a2lNames, statusMessage } from '$lib/stores';

  async function handleImportElf() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'ELF', extensions: ['elf', 'out', 'axf'] }]
    });
    if (selected) {
      isLoading.set(true);
      statusMessage.set('⏳ 正在加载...');
      try {
        const result = await loadElf(selected as string);
        elfPath.set(selected as string);
        elfFileName.set(result.meta.file_name);
        elfTotalCount.set(result.entry_count);
        packagePath.set((selected as string) + '.a2ldata');
        const entries = await searchElfEntries('', 0, 10000);
        elfEntries.set(entries);
        statusMessage.set(`✅ 已加载 ${result.entry_count} 个条目`);
      } catch (e) {
        statusMessage.set(`❌ 加载失败: ${e}`);
        // 如果数据包不存在，设置 elfPath 并显示生成对话框
        if (String(e).includes('数据包不存在')) {
          elfPath.set(selected as string);
          elfFileName.set((selected as string).split('/').pop() || '');
          showGenerateDialog.set(true);
        }
      }
      isLoading.set(false);
    }
  }

  async function handleImportPackage() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'A2L Data', extensions: ['a2ldata'] }]
    });
    if (selected) {
      isLoading.set(true);
      try {
        const result = await loadPackage(selected as string);
        packagePath.set(selected as string);
        elfPath.set(result.meta.elf_path || null);
        elfFileName.set(result.meta.file_name);
        elfTotalCount.set(result.entry_count);
        const entries = await searchElfEntries('', 0, 10000);
        elfEntries.set(entries);
        statusMessage.set(`✅ 已加载 ${result.entry_count} 个条目`);
      } catch (e) {
        statusMessage.set(`❌ 加载失败: ${e}`);
      }
      isLoading.set(false);
    }
  }

  async function handleImportA2l() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'A2L', extensions: ['a2l'] }]
    });
    if (selected) {
      isLoading.set(true);
      try {
        const result = await loadA2l(selected as string);
        a2lPath.set(selected as string);
        a2lNames.set(new Set(result.existing_names));
        const vars = await searchA2lVariables('', 0, 10000);
        a2lVariables.set(vars);
        statusMessage.set(`✅ 已加载目标 A2L (${result.variable_count} 个变量)`);
      } catch (e) {
        statusMessage.set(`❌ 加载 A2L 失败: ${e}`);
      }
      isLoading.set(false);
    }
  }

  function formatPath(path: string | null): string {
    if (!path) return '未选择';
    const parts = path.split('/');
    return parts[parts.length - 1] || path;
  }

  let elfDisplay = $derived($elfPath 
    ? `${formatPath($elfPath)} (${$elfTotalCount?.toLocaleString() ?? '0'} 条目)` 
    : '未选择');
  
  let packageDisplay = $derived($packagePath ? formatPath($packagePath) : '未选择');
  
  let a2lDisplay = $derived($a2lPath 
    ? `${formatPath($a2lPath)} (${$a2lVariables?.length?.toLocaleString() ?? '0'} 个变量)` 
    : '未选择');
</script>

<div class="file-info">
  <div class="row">
    <span class="icon">📂</span>
    <span class="label">ELF:</span>
    <span class="value" class:empty={!$elfPath}>{elfDisplay}</span>
    <button class="import-btn" onclick={handleImportElf}>导入</button>
  </div>
  <div class="row">
    <span class="icon">📦</span>
    <span class="label">数据包:</span>
    <span class="value" class:empty={!$packagePath}>{packageDisplay}</span>
    <button class="import-btn" onclick={handleImportPackage}>导入</button>
  </div>
  <div class="row">
    <span class="icon">📄</span>
    <span class="label">A2L:</span>
    <span class="value" class:empty={!$a2lPath}>{a2lDisplay}</span>
    <button class="import-btn" onclick={handleImportA2l}>导入</button>
  </div>
</div>

<style>
  .file-info {
    padding: 8px 16px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
  }

  .icon {
    width: 20px;
  }

  .label {
    min-width: 50px;
    color: var(--text-muted);
  }

  .value {
    flex: 1;
    font-family: monospace;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .value.empty {
    color: var(--text-muted);
  }

  .import-btn {
    padding: 4px 12px;
    background: var(--accent);
    border: none;
    border-radius: 4px;
    color: white;
    cursor: pointer;
    font-size: 12px;
  }

  .import-btn:hover {
    opacity: 0.9;
  }
</style>
