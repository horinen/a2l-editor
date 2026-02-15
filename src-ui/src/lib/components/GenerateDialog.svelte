<script lang="ts">
  import { showGenerateDialog, elfPath, elfFileName, elfTotalCount, packagePath, statusMessage, isLoading, elfEntries } from '$lib/stores';
  import { generatePackage, searchElfEntries } from '$lib/commands';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { fly } from 'svelte/transition';

  let customPath = $state<string | null>(null);

  async function handleGenerate() {
    if (!$elfPath) return;
    
    isLoading.set(true);
    statusMessage.set('⏳ 正在生成数据包...');
    
    try {
      const result = await generatePackage($elfPath, customPath || undefined);
      elfTotalCount.set(result.entry_count);
      packagePath.set(customPath || $elfPath + '.a2ldata');
      
      // 加载变量
      const entries = await searchElfEntries('', 0, 10000);
      elfEntries.set(entries);
      
      statusMessage.set(`✅ 数据包生成成功，已加载 ${result.entry_count} 个条目`);
    } catch (e) {
      statusMessage.set(`❌ 生成失败: ${e}`);
    }
    
    isLoading.set(false);
    showGenerateDialog.set(false);
    customPath = null;
  }

  async function handleSelectPath() {
    const selected = await save({
      filters: [{ name: 'A2L Data', extensions: ['a2ldata'] }],
      defaultPath: $elfPath ? $elfPath + '.a2ldata' : undefined
    });
    if (selected) {
      customPath = selected as string;
    }
  }

  function close() {
    showGenerateDialog.set(false);
    customPath = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  let displayPath = $derived(customPath || ($elfPath ? $elfPath + '.a2ldata' : '未选择'));
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $showGenerateDialog}
  <div class="overlay" onclick={close} role="dialog" aria-modal="true">
    <div class="dialog" transition:fly={{ duration: 150, y: -20 }} onclick={(e) => e.stopPropagation()}>
      <div class="header">
        <h3>生成数据包</h3>
        <button class="close-btn" onclick={close}>✖</button>
      </div>
      
      <div class="content">
        <p>ELF 文件: <strong>{$elfFileName || '未选择'}</strong></p>
        
        <div class="path-info">
          <p>数据包将保存到:</p>
          <code>{displayPath}</code>
        </div>
        
        <p class="warning">⚠️ 首次解析大型 ELF 可能需要几分钟</p>
      </div>
      
      <div class="footer">
        <button class="btn secondary" onclick={handleSelectPath}>选择其他位置</button>
        <button class="btn secondary" onclick={close}>取消</button>
        <button class="btn primary" onclick={handleGenerate}>生成</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    min-width: 400px;
    max-width: 520px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border);
  }

  .header h3 {
    margin: 0;
    font-size: 16px;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 18px;
  }

  .content {
    padding: 16px;
  }

  .path-info {
    margin-top: 12px;
    padding: 12px;
    background: var(--bg-hover);
    border-radius: 4px;
  }

  .path-info p {
    margin: 0 0 8px 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .path-info code {
    font-size: 12px;
    word-break: break-all;
  }

  .warning {
    margin-top: 12px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .btn.primary {
    background: var(--accent);
    color: white;
  }

  .btn.secondary {
    background: var(--bg-hover);
    color: var(--text);
  }
</style>
