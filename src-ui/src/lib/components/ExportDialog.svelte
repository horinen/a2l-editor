<script lang="ts">
  import { showExportDialog, exportMode, exportPreview, statusMessage, elfSelectedIndices } from '$lib/stores';
  import { exportEntries } from '$lib/commands';
  import { fly } from 'svelte/transition';

  async function handleExport() {
    try {
      const indices = Array.from($elfSelectedIndices);
      const result = await exportEntries(indices, $exportMode);
      statusMessage.set(`✅ 已添加 ${result.added} 个变量到目标 A2L`);
    } catch (e) {
      statusMessage.set(`❌ 导出失败: ${e}`);
    }
    showExportDialog.set(false);
  }

  function close() {
    showExportDialog.set(false);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $showExportDialog}
  <div class="overlay" onclick={close} role="dialog" aria-modal="true">
    <div class="dialog" transition:fly={{ duration: 150, y: -20 }} onclick={(e) => e.stopPropagation()}>
      <div class="header">
        <h3>导出到 A2L</h3>
        <button class="close-btn" onclick={close}>✖</button>
      </div>
      
      <div class="content">
        <p>将 <strong>{$exportPreview?.added || 0}</strong> 个变量添加为<strong>{$exportMode === 'measurement' ? '观测' : '标定'}变量</strong></p>
        
        {#if $exportPreview}
          <div class="preview">
            <div class="row"><span>新增:</span><span>{$exportPreview.added}</span></div>
            <div class="row"><span>跳过 (已存在):</span><span>{$exportPreview.skipped}</span></div>
            <div class="row"><span>目标文件已有:</span><span>{$exportPreview.existing}</span></div>
          </div>
        {/if}
      </div>
      
      <div class="footer">
        <button class="btn secondary" onclick={close}>取消</button>
        <button class="btn primary" onclick={handleExport}>确认追加</button>
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
    min-width: 360px;
    max-width: 480px;
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

  .preview {
    margin-top: 12px;
    padding: 12px;
    background: var(--bg-hover);
    border-radius: 4px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
    font-size: 13px;
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
