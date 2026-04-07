<script lang="ts">
  import { fly } from 'svelte/transition';
  import { currentTheme, a2lPath, endianness } from '$lib/stores';
  import { themes, themeNames, applyTheme, cycleTheme } from '$lib/themes';
  import { showAboutDialog, showGenerateDialog, showHelpDialog, statusMessage, isLoading } from '$lib/stores';
  import { open } from '@tauri-apps/plugin-dialog';
  import { loadElf, loadPackage, loadA2l, setEndianness, updateA2lAddresses } from '$lib/commands';
  import { 
    elfPath, elfFileName, elfTotalCount, elfEntries,
    packagePath, a2lVariables, a2lNames
  } from '$lib/stores';
  import { searchA2lVariables } from '$lib/commands';

  import { getVersion } from '@tauri-apps/api/app';

  let showMenu = $state(false);
  let version = $state('');

  $effect(() => {
    getVersion().then(v => version = 'v' + v);
  });

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

  async function handleUpdateAddresses() {
    if (!$a2lPath) {
      statusMessage.set('❌ 请先选择目标 A2L 文件');
      showMenu = false;
      return;
    }
    if (!$elfPath) {
      statusMessage.set('❌ 请先加载 ELF 文件');
      showMenu = false;
      return;
    }
    isLoading.set(true);
    statusMessage.set('⏳ 正在更新地址...');
    try {
      const result = await updateA2lAddresses();
      const vars = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(vars);
      statusMessage.set(`✅ 已更新 ${result.updated} 个地址，跳过 ${result.skipped} 个`);
    } catch (e) {
      statusMessage.set(`❌ 更新失败: ${e}`);
    }
    isLoading.set(false);
    showMenu = false;
  }

  function closeMenu() {
    showMenu = false;
  }
</script>

<svelte:window onclick={() => showMenu = false} />

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
          <button onclick={handleUpdateAddresses}>🔄 更新 A2L 地址</button>
          <button onclick={() => { showGenerateDialog.set(true); showMenu = false; }}>🔄 重新生成缓存</button>
        </div>
      {/if}
    </div>
    <button class="icon-btn" onclick={() => showHelpDialog.set(true)}>❓ 手册</button>
    <button class="icon-btn" onclick={() => showAboutDialog.set(true)}>ℹ️ 关于</button>
  </div>
  <div class="right">
    <button class="icon-btn endianness-btn" onclick={handleToggleEndianness} title="切换字节序">
      {$endianness === 'little' ? '小端' : '大端'}
    </button>
    <button class="icon-btn theme-btn" onclick={handleCycleTheme} title="切换主题">🎨</button>
    <span class="version">{version}</span>
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
