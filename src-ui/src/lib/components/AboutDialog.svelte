<script lang="ts">
  import { showAboutDialog } from '$lib/stores';
  import { fly } from 'svelte/transition';

  const VERSION = 'v0.1.0';

  function close() {
    showAboutDialog.set(false);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $showAboutDialog}
  <div class="overlay" onclick={close} role="dialog" aria-modal="true">
    <div class="dialog" transition:fly={{ duration: 150, y: -20 }} onclick={(e) => e.stopPropagation()}>
      <div class="header">
        <h3>关于 A2L Editor</h3>
        <button class="close-btn" onclick={close}>✖</button>
      </div>
      
      <div class="content">
        <div class="logo">
          <h2>A2L Editor</h2>
          <p class="version">{VERSION}</p>
        </div>
        
        <p class="desc">从 ELF/DWARF 生成 A2L 文件的桌面工具</p>
        
        <div class="info">
          <p><strong>技术栈:</strong> Rust + Tauri + Svelte</p>
          <p><strong>仓库:</strong> <a href="https://github.com/horinen/a2l-editor" target="_blank">github.com/horinen/a2l-editor</a></p>
        </div>
      </div>
      
      <div class="footer">
        <button class="btn primary" onclick={close}>确定</button>
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
    min-width: 320px;
    max-width: 400px;
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
    padding: 24px 16px;
    text-align: center;
  }

  .logo h2 {
    margin: 0;
    font-size: 24px;
  }

  .version {
    color: var(--text-muted);
    font-size: 14px;
    margin: 8px 0 0 0;
  }

  .desc {
    margin: 16px 0;
    color: var(--text-muted);
    font-size: 14px;
  }

  .info {
    text-align: left;
    padding: 12px;
    background: var(--bg-hover);
    border-radius: 4px;
    font-size: 13px;
  }

  .info p {
    margin: 4px 0;
  }

  .info a {
    color: var(--accent);
    text-decoration: none;
  }

  .footer {
    display: flex;
    justify-content: center;
    padding: 16px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 24px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .btn.primary {
    background: var(--accent);
    color: white;
  }
</style>
