<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
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
  import ContextMenuA2l from '$lib/components/ContextMenuA2l.svelte';
  import ContextMenuElf from '$lib/components/ContextMenuElf.svelte';
  import LoadingOverlay from '$lib/components/LoadingOverlay.svelte';
  import CloseConfirmDialog from '$lib/components/CloseConfirmDialog.svelte';
  import { setupAutoLoad, testLoadFiles } from '$lib/autoLoad';
  import { 
    elfSelectedIndices, a2lSelectedIndices, a2lPath, elfEntries,
    statusMessage, showExportDialog, exportMode, exportPreview, a2lVariables,
    hasUnsavedChanges, pendingChanges, clearPendingChanges, requestCloseConfirm
  } from '$lib/stores';
  import { exportEntries, deleteVariables, searchA2lVariables, saveA2lChanges } from '$lib/commands';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { tick } from 'svelte';

  const appWindow = getCurrentWindow();
  let isClosingConfirmed = false;

  onMount(() => {
    setupAutoLoad();
    (window as any).__test_loadFiles__ = testLoadFiles;

    const unlisten = appWindow.listen('close-requested', async () => {
      if (isClosingConfirmed) {
        await appWindow.destroy();
        return;
      }
      
      if ($hasUnsavedChanges) {
        requestCloseConfirm(async (save: boolean) => {
          if (save) {
            try {
              const changes = $pendingChanges;
              await saveA2lChanges(changes);
              clearPendingChanges();
            } catch (e) {
              console.error('保存失败:', e);
            }
          }
          isClosingConfirmed = true;
          await appWindow.destroy();
        });
      } else {
        isClosingConfirmed = true;
        await appWindow.destroy();
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  });

  let a2lPanelRef: A2lPanel | undefined;

  let contextMenu = $state<{ show: boolean; x: number; y: number; indices: number[]; type: 'elf' | 'a2l' }>({
    show: false,
    x: 0,
    y: 0,
    indices: [],
    type: 'elf'
  });

  function handleA2lContextMenu(e: CustomEvent<{ x: number; y: number; indices: number[] }>) {
    contextMenu = { show: true, x: e.detail.x, y: e.detail.y, indices: e.detail.indices, type: 'a2l' };
  }

  function handleElfContextMenu(e: CustomEvent<{ x: number; y: number; indices: number[] }>) {
    contextMenu = { show: true, x: e.detail.x, y: e.detail.y, indices: e.detail.indices, type: 'elf' };
  }

  function closeContextMenu() {
    contextMenu = { ...contextMenu, show: false };
  }

  async function handleExport(e: CustomEvent<{ indices: number[]; mode: string }>) {
    const exportedNames = e.detail.indices
      .map(i => $elfEntries.find(entry => entry.index === i)?.full_name)
      .filter(Boolean) as string[];
    
    try {
      const result = await exportEntries(e.detail.indices, e.detail.mode as 'measurement' | 'characteristic');
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

  async function handleDelete(e: CustomEvent<{ indices: number[] }>) {
    try {
      const count = await deleteVariables(e.detail.indices);
      statusMessage.set(`✅ 已删除 ${count} 个变量`);
    } catch (err) {
      statusMessage.set(`❌ 删除失败: ${err}`);
    }
    closeContextMenu();
  }

  async function handleCopyNames(e: CustomEvent<{ indices: number[] }>) {
    const names = e.detail.indices.map(i => {
      if (contextMenu.type === 'elf') {
        return $elfEntries.find(entry => entry.index === i)?.full_name;
      } else {
        return $a2lVariables[i]?.name;
      }
    }).filter(Boolean).join('\n');
    
    try {
      await writeText(names);
      statusMessage.set('✅ 已复制名称到剪贴板');
    } catch (err) {
      statusMessage.set(`❌ 复制失败: ${err}`);
    }
    closeContextMenu();
  }

  async function handleCopyAddresses(e: CustomEvent<{ indices: number[] }>) {
    const addresses = e.detail.indices.map(i => {
      if (contextMenu.type === 'elf') {
        const entry = $elfEntries.find(en => en.index === i);
        return entry ? `0x${entry.address.toString(16).toUpperCase().padStart(8, '0')}` : '';
      } else {
        return $a2lVariables[i]?.address || '';
      }
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
      a2lSelectedIndices.set(new Set());
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
<LoadingOverlay />
<CloseConfirmDialog />

{#if contextMenu.show && contextMenu.type === 'a2l'}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <ContextMenuA2l 
    x={contextMenu.x} 
    y={contextMenu.y} 
    indices={contextMenu.indices}
    ondelete={handleDelete}
    oncopyNames={handleCopyNames}
    oncopyAddresses={handleCopyAddresses}
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
    oncopyNames={handleCopyNames}
    oncopyAddresses={handleCopyAddresses}
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
