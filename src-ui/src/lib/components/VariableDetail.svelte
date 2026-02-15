<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { A2lEntry, A2lVariable } from '$lib/types';

  interface Props {
    entry?: A2lEntry;
    variable?: A2lVariable;
    x: number;
    y: number;
    visible: boolean;
  }
  
  let { entry, variable, x, y, visible }: Props = $props();
  
  let popupX = $state(x);
  let popupY = $state(y);
  let popupElement: HTMLDivElement;

  $effect(() => {
    if (!popupElement || !visible) return;
    
    const rect = popupElement.getBoundingClientRect();
    const padding = 12;
    
    let newX = x + padding;
    let newY = y + padding;
    
    if (newX + rect.width > window.innerWidth - padding) {
      newX = x - rect.width - padding;
    }
    if (newY + rect.height > window.innerHeight - padding) {
      newY = y - rect.height - padding;
    }
    if (newX < padding) newX = padding;
    if (newY < padding) newY = padding;
    
    popupX = newX;
    popupY = newY;
  });

  function formatAddress(addr: number | undefined | null): string {
    if (addr === undefined || addr === null) return 'N/A';
    return '0x' + addr.toString(16).toUpperCase().padStart(8, '0');
  }
</script>

{#if visible && (entry || variable)}
  <div 
    class="detail-popup" 
    bind:this={popupElement}
    style="left: {popupX}px; top: {popupY}px;"
    transition:fade={{ duration: 150 }}
  >
    {#if entry}
      <div class="section">
        <div class="label">完整名称</div>
        <div class="value name">{entry.full_name}</div>
      </div>
      <div class="row">
        <div class="section">
          <div class="label">地址</div>
          <div class="value">{formatAddress(entry.address)}</div>
        </div>
        <div class="section">
          <div class="label">大小</div>
          <div class="value">{entry.size} 字节</div>
        </div>
      </div>
      <div class="row">
        <div class="section">
          <div class="label">A2L 类型</div>
          <div class="value">{entry.a2l_type}</div>
        </div>
        <div class="section">
          <div class="label">原始类型</div>
          <div class="value">{entry.type_name}</div>
        </div>
      </div>
      {#if entry.bit_offset !== null && entry.bit_size !== null}
        <div class="row">
          <div class="section">
            <div class="label">位偏移</div>
            <div class="value">{entry.bit_offset}</div>
          </div>
          <div class="section">
            <div class="label">位大小</div>
            <div class="value">{entry.bit_size}</div>
          </div>
        </div>
      {/if}
    {:else if variable}
      <div class="section">
        <div class="label">变量名</div>
        <div class="value name">{variable.name}</div>
      </div>
      <div class="row">
        <div class="section">
          <div class="label">地址</div>
          <div class="value">{variable.address || 'N/A'}</div>
        </div>
        <div class="section">
          <div class="label">数据类型</div>
          <div class="value">{variable.data_type}</div>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .detail-popup {
    position: fixed;
    z-index: 9000;
    min-width: 280px;
    max-width: 400px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    padding: 12px;
    pointer-events: none;
  }
  
  .section {
    flex: 1;
    min-width: 0;
  }
  
  .row {
    display: flex;
    gap: 16px;
  }
  
  .label {
    font-size: 11px;
    color: var(--text-muted);
    margin-bottom: 2px;
  }
  
  .value {
    font-family: monospace;
    font-size: 13px;
    color: var(--text);
  }
  
  .value.name {
    word-break: break-all;
    font-size: 12px;
  }
  
  .section + .section {
    margin-top: 8px;
  }
  
  .row + .row {
    margin-top: 8px;
  }
</style>
