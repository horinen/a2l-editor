<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import Header from '$lib/components/Header.svelte';
  import FileInfo from '$lib/components/FileInfo.svelte';
  import A2lPanel from '$lib/components/A2lPanel.svelte';
  import VariableList from '$lib/components/VariableList.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import ExportDialog from '$lib/components/ExportDialog.svelte';
  import GenerateDialog from '$lib/components/GenerateDialog.svelte';
  import AboutDialog from '$lib/components/AboutDialog.svelte';
  import HelpDialog from '$lib/components/HelpDialog.svelte';
  import CompuMethodPanel from '$lib/components/CompuMethodPanel.svelte';
  import ContextMenuA2l from '$lib/components/ContextMenuA2l.svelte';
  import ContextMenuElf from '$lib/components/ContextMenuElf.svelte';
  import LoadingOverlay from '$lib/components/LoadingOverlay.svelte';
  import { setupAutoLoad, testLoadFiles } from '$lib/autoLoad';
  import { handleStalePackage } from '$lib/stalePackage';
  import { 
    elfSelectedIndices, a2lSelectedNames, a2lPath, elfEntries,
    statusMessage, a2lVariables, elfPath, elfFileName, elfTotalCount,
    packagePath, a2lNames, isLoading
  } from '$lib/stores';
  import { exportEntries, deleteVariables, searchA2lVariables, searchElfEntries, loadElf, loadA2l } from '$lib/commands';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { tick } from 'svelte';
  import {
    getRecentElfFiles, removeRecentElfFile, addRecentElfFile,
    getRecentA2lFiles, removeRecentA2lFile, addRecentA2lFile
  } from '$lib/recentFiles';

  let autoLoaded = false;

  onMount(() => {
    setupAutoLoad(() => { autoLoaded = true; });
    (window as any).__test_loadFiles__ = testLoadFiles;

    setTimeout(async () => {
      if (autoLoaded) return;

      const recentElf = getRecentElfFiles();
      const recentA2l = getRecentA2lFiles();

      if (recentElf.length > 0) {
        autoLoaded = true;
        isLoading.set(true);
        statusMessage.set('⏳ 正在恢复上次 ELF...');
        try {
          const path = recentElf[0].path;
          const result = await loadElf(path);
          if (result.status === 'stale') {
            // 缓存过期：弹出带原因提示的生成对话框（升级后首次启动的高频场景）
            handleStalePackage(result);
          } else {
            elfPath.set(path);
            elfFileName.set(result.meta.file_name);
            elfTotalCount.set(result.entry_count);
            packagePath.set(path + '.a2ldata');
            const entries = await searchElfEntries('', 0, 10000);
            elfEntries.set(entries);
            statusMessage.set(`✅ 已恢复 ${result.entry_count} 个条目`);
          }
        } catch (e) {
          if (String(e).includes('数据包不存在')) {
            elfPath.set(recentElf[0].path);
            elfFileName.set(recentElf[0].name);
            packagePath.set(recentElf[0].path + '.a2ldata');
            statusMessage.set('⚠️ 上次 ELF 的数据包不存在，请重新生成');
          } else {
            statusMessage.set(`⚠️ 上次 ELF 文件已不可用: ${e}`);
            removeRecentElfFile(recentElf[0].path);
          }
        }
      }

      if (recentA2l.length > 0) {
        autoLoaded = true;
        isLoading.set(true);
        statusMessage.update(prev => prev.includes('已恢复') ? prev : '⏳ 正在恢复上次 A2L...');
        try {
          const path = recentA2l[0].path;
          const result = await loadA2l(path);
          a2lPath.set(path);
          a2lNames.set(new Set(result.existing_names));
          const vars = await searchA2lVariables('', 0, 10000);
          a2lVariables.set(vars);
          statusMessage.update(prev => {
            if (prev.includes('已恢复') && prev.includes('条目')) {
              return prev + `，A2L (${result.variable_count} 个变量)`;
            }
            return `✅ 已恢复 A2L (${result.variable_count} 个变量)`;
          });
        } catch (e) {
          statusMessage.set(`⚠️ 上次 A2L 文件已不可用: ${e}`);
          removeRecentA2lFile(recentA2l[0].path);
        }
      }

      isLoading.set(false);
    }, 200);
  });

  let a2lPanelRef: A2lPanel | undefined;

  let contextMenu = $state<{ show: boolean; x: number; y: number; names: string[]; indices: number[]; type: 'elf' | 'a2l' }>({
    show: false,
    x: 0,
    y: 0,
    names: [],
    indices: [],
    type: 'elf'
  });

  function handleA2lContextMenu(e: CustomEvent<{ x: number; y: number; names: string[] }>) {
    contextMenu = { show: true, x: e.detail.x, y: e.detail.y, names: e.detail.names, indices: [], type: 'a2l' };
  }

  function handleElfContextMenu(e: CustomEvent<{ x: number; y: number; indices: number[] }>) {
    contextMenu = { show: true, x: e.detail.x, y: e.detail.y, names: [], indices: e.detail.indices, type: 'elf' };
  }

  function closeContextMenu() {
    contextMenu = { ...contextMenu, show: false };
  }

  async function handleExport(e: CustomEvent<{ indices: number[]; mode: string }>) {
    const mode = e.detail.mode as 'measurement' | 'characteristic';
    const exportedNames = e.detail.indices
      .map(i => $elfEntries.find(entry => entry.index === i)?.full_name)
      .filter(Boolean) as string[];
    
    try {
      const result = await exportEntries(e.detail.indices, mode);
      statusMessage.set(`✅ 已添加 ${result.added} 个变量到目标 A2L`);
      
      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);
      
      if (exportedNames.length > 0 && result.added > 0) {
        await tick();
        if (a2lPanelRef) {
          for (const name of exportedNames) {
            if (a2lPanelRef.scrollToVariable(name)) {
              break;
            }
          }
        }
      }
    } catch (err) {
      statusMessage.set(`❌ 导出失败: ${err}`);
    }
    closeContextMenu();
  }

  async function handleDelete(e: CustomEvent<{ names: string[] }>) {
    try {
      const count = await deleteVariables(e.detail.names);
      statusMessage.set(`✅ 已删除 ${count} 个变量`);
      
      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);
    } catch (err) {
      statusMessage.set(`❌ 删除失败: ${err}`);
    }
    closeContextMenu();
  }

  // A2L 复制名称
  async function handleA2lCopyNames(e: CustomEvent<{ names: string[] }>) {
    const names = e.detail.names.join('\n');
    
    try {
      await writeText(names);
      statusMessage.set('✅ 已复制名称到剪贴板');
    } catch (err) {
      statusMessage.set(`❌ 复制失败: ${err}`);
    }
    closeContextMenu();
  }

  // ELF 复制名称
  async function handleElfCopyNames(e: CustomEvent<{ indices: number[] }>) {
    const names = e.detail.indices
      .map(i => $elfEntries.find(entry => entry.index === i)?.full_name)
      .filter(Boolean).join('\n');
    
    try {
      await writeText(names);
      statusMessage.set('✅ 已复制名称到剪贴板');
    } catch (err) {
      statusMessage.set(`❌ 复制失败: ${err}`);
    }
    closeContextMenu();
  }

  // A2L 复制地址
  async function handleA2lCopyAddresses(e: CustomEvent<{ names: string[] }>) {
    const addresses = e.detail.names.map(name => {
      const variable = $a2lVariables.find(v => v.name === name);
      return variable?.address || '';
    }).filter(Boolean).join('\n');
    
    try {
      await writeText(addresses);
      statusMessage.set('✅ 已复制地址到剪贴板');
    } catch (err) {
      statusMessage.set(`❌ 复制失败: ${err}`);
    }
    closeContextMenu();
  }

  // ELF 复制地址
  async function handleElfCopyAddresses(e: CustomEvent<{ indices: number[] }>) {
    const addresses = e.detail.indices.map(i => {
      const entry = $elfEntries.find(en => en.index === i);
      return entry ? `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}` : '';
    }).filter(Boolean).join('\n');
    
    try {
      await writeText(addresses);
      statusMessage.set('✅ 已复制地址到剪贴板');
    } catch (err) {
      statusMessage.set(`❌ 复制失败: ${err}`);
    }
    closeContextMenu();
  }

  function handleClearSelection() {
    if (contextMenu.type === 'elf') {
      elfSelectedIndices.set(new Set());
    } else {
      a2lSelectedNames.set(new Set());
    }
    closeContextMenu();
  }

  let leftWidth = $state(
    typeof window !== 'undefined' 
      ? parseFloat(localStorage.getItem('a2l-editor-panel-width') || '50')
      : 50
  );

  function handleResize(e: MouseEvent) {
    const container = document.querySelector('.panels');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    leftWidth = ((e.clientX - rect.left) / rect.width) * 100;
    leftWidth = Math.max(20, Math.min(80, leftWidth));
    localStorage.setItem('a2l-editor-panel-width', leftWidth.toString());
  }
</script>

<svelte:head>
  <title>A2L Editor</title>
</svelte:head>

<main class="h-screen flex flex-col">
  <Header />
  <FileInfo />
  
  <div class="panels flex-1 flex overflow-hidden">
    <div class="panel-left" style="width: {leftWidth}%">
      <A2lPanel bind:this={a2lPanelRef} oncontextmenu={handleA2lContextMenu} />
    </div>
    
    <div class="resizer" onmousedown={() => {
      document.addEventListener('mousemove', handleResize);
      document.addEventListener('mouseup', () => {
        document.removeEventListener('mousemove', handleResize);
      }, { once: true });
    }}></div>
    
    <div class="panel-right" style="width: {100 - leftWidth}%">
      <VariableList oncontextmenu={handleElfContextMenu} />
    </div>
  </div>
  
  <StatusBar />
</main>

<ExportDialog />
<GenerateDialog />
<AboutDialog />
<HelpDialog />
<CompuMethodPanel />
<LoadingOverlay />

{#if contextMenu.show && contextMenu.type === 'a2l'}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <ContextMenuA2l 
    x={contextMenu.x} 
    y={contextMenu.y} 
    names={contextMenu.names}
    ondelete={handleDelete}
    oncopyNames={handleA2lCopyNames}
    oncopyAddresses={handleA2lCopyAddresses}
    onclear={handleClearSelection}
    onclose={closeContextMenu}
  />
{/if}

{#if contextMenu.show && contextMenu.type === 'elf'}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <ContextMenuElf 
    x={contextMenu.x} 
    y={contextMenu.y} 
    indices={contextMenu.indices}
    onexport={handleExport}
    oncopyNames={handleElfCopyNames}
    oncopyAddresses={handleElfCopyAddresses}
    onclear={handleClearSelection}
    onclose={closeContextMenu}
  />
{/if}

<style>
  .panels {
    flex: 1;
    overflow: hidden;
  }

  .panel-left, .panel-right {
    height: 100%;
    overflow: hidden;
  }

  .resizer {
    width: 6px;
    cursor: col-resize;
    background: var(--border);
    transition: background 0.2s;
  }

  .resizer:hover {
    background: var(--accent);
  }
</style>
