<script lang="ts">
  import { fly } from 'svelte/transition';
  import { showCloseConfirmDialog, confirmClose, changeCount } from '$lib/stores';

  function handleSave() {
    confirmClose(true);
  }

  function handleDontSave() {
    confirmClose(false);
  }

  function handleCancel() {
    showCloseConfirmDialog.set(false);
  }
</script>

{#if $showCloseConfirmDialog}
  <div class="overlay" onclick={handleCancel}>
    <div class="dialog" transition:fly={{ duration: 150, y: -20 }} onclick={(e) => e.stopPropagation()}>
      <div class="dialog-header">
        <span class="icon">⚠️</span>
        <span class="title">未保存的更改</span>
      </div>
      
      <div class="dialog-body">
        <p>您有 <strong>{$changeCount}</strong> 个未保存的更改。</p>
        <p>关闭前是否保存？</p>
      </div>
      
      <div class="dialog-footer">
        <button class="btn btn-secondary" onclick={handleCancel}>取消</button>
        <button class="btn btn-danger" onclick={handleDontSave}>不保存</button>
        <button class="btn btn-primary" onclick={handleSave}>保存</button>
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
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    min-width: 320px;
    max-width: 400px;
  }

  .dialog-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .icon {
    font-size: 20px;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
  }

  .dialog-body {
    padding: 20px;
    color: var(--text);
    font-size: 14px;
    line-height: 1.6;
  }

  .dialog-body p {
    margin: 0 0 8px 0;
  }

  .dialog-body p:last-child {
    margin-bottom: 0;
  }

  .dialog-body strong {
    color: #f59e0b;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    transition: all 0.2s;
  }

  .btn:hover {
    background: var(--bg-hover);
  }

  .btn-primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }

  .btn-danger {
    color: #ef4444;
    border-color: #ef4444;
  }

  .btn-danger:hover {
    background: #ef4444;
    color: white;
  }
</style>
