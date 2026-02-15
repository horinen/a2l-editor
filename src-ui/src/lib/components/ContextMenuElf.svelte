<script lang="ts">
  import { fly } from 'svelte/transition';
  import { a2lPath } from '$lib/stores';

  interface Props {
    x: number;
    y: number;
    indices: number[];
    onexport?: (e: CustomEvent<{ indices: number[]; mode: string }>) => void;
    oncopyNames?: (e: CustomEvent<{ indices: number[] }>) => void;
    oncopyAddresses?: (e: CustomEvent<{ indices: number[] }>) => void;
    onclear?: (e: CustomEvent) => void;
    onclose?: (e: CustomEvent) => void;
  }
  
  let { x, y, indices, onexport, oncopyNames, oncopyAddresses, onclear, onclose }: Props = $props();

  let menuElement: HTMLDivElement;
  let menuX = $state(x);
  let menuY = $state(y);

  $effect(() => {
    if (!menuElement) return;
    
    const rect = menuElement.getBoundingClientRect();
    const padding = 8;
    
    let newX = x;
    let newY = y;
    
    if (rect.right > window.innerWidth - padding) {
      newX = window.innerWidth - rect.width - padding;
    }
    if (rect.bottom > window.innerHeight - padding) {
      newY = window.innerHeight - rect.height - padding;
    }
    if (newX < padding) newX = padding;
    if (newY < padding) newY = padding;
    
    menuX = newX;
    menuY = newY;
  });

  function exportAsMeasurement() {
    onexport?.(new CustomEvent('export', { detail: { indices, mode: 'measurement' } }));
    onclose?.(new CustomEvent('close'));
  }

  function exportAsCharacteristic() {
    onexport?.(new CustomEvent('export', { detail: { indices, mode: 'characteristic' } }));
    onclose?.(new CustomEvent('close'));
  }

  function copyNames() {
    oncopyNames?.(new CustomEvent('copyNames', { detail: { indices } }));
    onclose?.(new CustomEvent('close'));
  }

  function copyAddresses() {
    oncopyAddresses?.(new CustomEvent('copyAddresses', { detail: { indices } }));
    onclose?.(new CustomEvent('close'));
  }

  function clearSelection() {
    onclear?.(new CustomEvent('clear'));
    onclose?.(new CustomEvent('close'));
  }

  function close() {
    onclose?.(new CustomEvent('close'));
  }
</script>

<svelte:window onclick={close} onkeydown={(e) => e.key === 'Escape' && close()} />

<div class="menu" bind:this={menuElement} style="left: {menuX}px; top: {menuY}px;" transition:fly={{ duration: 100, y: -5 }}>
  <button class="item" disabled={!a2lPath} onclick={exportAsMeasurement}>📊 添加为观测变量</button>
  <button class="item" disabled={!a2lPath} onclick={exportAsCharacteristic}>📈 添加为标定变量</button>
  <div class="divider"></div>
  <button class="item" onclick={copyNames}>📋 复制名称</button>
  <button class="item" onclick={copyAddresses}>📋 复制地址</button>
  <div class="divider"></div>
  <button class="item" onclick={clearSelection}>✖ 取消选择</button>
</div>

<style>
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
  }

  .item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }

  .item:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .item:disabled {
    color: var(--text-muted);
    cursor: not-allowed;
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }
</style>
