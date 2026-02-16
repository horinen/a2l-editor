<script lang="ts">
  import { fly } from 'svelte/transition';
  import { currentTheme, hasUnsavedChanges, pendingChanges, a2lPath, endianness } from '$lib/stores';
  import { themes, themeNames, applyTheme, cycleTheme } from '$lib/themes';
  import { showAboutDialog, showGenerateDialog, showHelpDialog, statusMessage, isLoading, clearPendingChanges } from '$lib/stores';
  import { open } from '@tauri-apps/plugin-dialog';
  import { loadElf, loadPackage, loadA2l, saveA2lChanges, setEndianness } from '$lib/commands';
  import { 
    elfPath, elfFileName, elfTotalCount, elfEntries,
    packagePath, a2lVariables, a2lNames,
    showExportDialog, exportPreview
  } from '$lib/stores';
  import { searchA2lVariables } from '$lib/commands';

  let showMenu = $state(false);
  let isSaving = $state(false);

  const VERSION = 'v0.1.0';

  async function handleSave() {
    if (!$hasUnsavedChanges || !$a2lPath || isSaving) return;
    
    isSaving = true;
    statusMessage.set('⏳ 正在保存...');
    
    try {
      const changes = $pendingChanges;
      const result = await saveA2lChanges(changes);
      
      const total = result.modified + result.deleted + result.added;
      statusMessage.set(`✅ 已保存 ${total} 个更改 (修改:${result.modified} 删除:${result.deleted} 添加:${result.added})`);
      
      clearPendingChanges();
      
      const vars = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(vars);
    } catch (e) {
      statusMessage.set(`❌ 保存失败: ${e}`);
    }
    
    isSaving = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      handleSave();
    }
  }

  async function handleOpenElf() {
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
        
        const entries = await (await import('$lib/commands')).searchElfEntries('', 0, 10000);
        elfEntries.set(entries);
        
        statusMessage.set(`✅ 已加载 ${result.entry_count} 个条目`);
      } catch (e) {
        statusMessage.set(`❌ 加载失败: ${e}`);
        if (String(e).includes('数据包不存在')) {
          elfPath.set(selected as string);
          elfFileName.set((selected as string).split('/').pop() || '');
          showGenerateDialog.set(true);
        }
      }
      isLoading.set(false);
    }
    showMenu = false;
  }

  async function handleOpenPackage() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'A2L Data', extensions: ['a2ldata'] }]
    });
    if (selected) {
      isLoading.set(true);
      statusMessage.set('⏳ 正在加载数据包...');
      try {
        const result = await loadPackage(selected as string);
        packagePath.set(selected as string);
        elfPath.set(result.meta.elf_path || null);
        elfFileName.set(result.meta.file_name);
        elfTotalCount.set(result.entry_count);
        
        const entries = await (await import('$lib/commands')).searchElfEntries('', 0, 10000);
        elfEntries.set(entries);
        
        statusMessage.set(`✅ 已加载 ${result.entry_count} 个条目`);
      } catch (e) {
        statusMessage.set(`❌ 加载失败: ${e}`);
      }
      isLoading.set(false);
    }
    showMenu = false;
  }

  async function handleSelectA2l() {
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
        
        const vars = await (await import('$lib/commands')).searchA2lVariables('', 0, 10000);
        a2lVariables.set(vars);
        
        statusMessage.set(`✅ 已加载目标 A2L (${result.variable_count} 个变量)`);
      } catch (e) {
        statusMessage.set(`❌ 加载 A2L 失败: ${e}`);
      }
      isLoading.set(false);
    }
    showMenu = false;
  }

  function handleCycleTheme() {
    const next = cycleTheme($currentTheme);
    currentTheme.set(next);
    applyTheme(next);
  }

  async function handleToggleEndianness() {
    const next = $endianness === 'little' ? 'big' : 'little';
    endianness.set(next);
    await setEndianness(next);
  }

  function closeMenu() {
    showMenu = false;
  }
</script>

<svelte:window onclick={() => showMenu = false} onkeydown={handleKeydown} />

<header class="header">
  <div class="left">
    <div class="dropdown">
      <button class="menu-btn" onclick={(e) => { e.stopPropagation(); showMenu = !showMenu; }}>
        📁 文件 ▼
      </button>
      {#if showMenu}
        <div class="menu" transition:fly={{ duration: 100, y: -5 }} onfocusout={closeMenu}>
          <button onclick={handleOpenElf}>📂 打开 ELF...</button>
          <button onclick={handleOpenPackage}>📦 打开数据包...</button>
          <button onclick={handleSelectA2l}>📄 选择目标 A2L...</button>
          <div class="divider"></div>
          <button onclick={() => { showGenerateDialog.set(true); showMenu = false; }}>🔄 重新生成缓存</button>
        </div>
      {/if}
    </div>
    <button class="icon-btn" onclick={() => showHelpDialog.set(true)}>❓ 手册</button>
    <button class="icon-btn" onclick={() => showAboutDialog.set(true)}>ℹ️ 关于</button>
  </div>
  <div class="right">
    {#if $a2lPath}
      <button 
        class="save-btn" 
        class:has-changes={$hasUnsavedChanges}
        class:saving={isSaving}
        onclick={handleSave}
        disabled={!$hasUnsavedChanges || isSaving}
        title="保存 (Ctrl+S)"
      >
        💾 保存
        {#if $hasUnsavedChanges}
          <span class="badge">{$pendingChanges.length}</span>
        {/if}
      </button>
    {/if}
    <button class="icon-btn endianness-btn" onclick={handleToggleEndianness} title="切换字节序">
      {$endianness === 'little' ? '小端' : '大端'}
    </button>
    <button class="icon-btn theme-btn" onclick={handleCycleTheme} title="切换主题">🎨</button>
    <span class="version">{VERSION}</span>
  </div>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }

  .left, .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dropdown {
    position: relative;
  }

  .menu-btn, .icon-btn {
    padding: 6px 12px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
  }

  .menu-btn:hover, .icon-btn:hover {
    background: var(--bg-hover);
  }

  .save-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 13px;
    transition: all 0.2s;
  }

  .save-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .save-btn.has-changes {
    color: var(--text);
    border-color: var(--accent);
  }

  .save-btn.has-changes:hover:not(:disabled) {
    background: var(--accent);
    color: white;
  }

  .save-btn.saving {
    opacity: 0.7;
    cursor: wait;
  }

  .save-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    background: var(--accent);
    color: white;
    font-size: 11px;
    font-weight: 600;
    border-radius: 9px;
  }

  .menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 4px;
    min-width: 180px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    z-index: 1000;
  }

  .menu button {
    display: block;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font-size: 13px;
  }

  .menu button:hover {
    background: var(--bg-hover);
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }

  .version {
    font-size: 12px;
    color: var(--text-muted);
    margin-left: 8px;
  }

  .endianness-btn {
    min-width: 48px;
    text-align: center;
  }
</style>
