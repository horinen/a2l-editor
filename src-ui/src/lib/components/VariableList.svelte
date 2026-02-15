<script lang="ts">
  import { 
    elfEntries, elfTotalCount, elfSearchQuery, elfSelectedIndices, a2lNames, 
    toggleElfSelection, isLoading, elfSortConfigs, toggleSort
  } from '$lib/stores';
  import type { A2lEntry } from '$lib/types';
  import type { SortField, SortConfig } from '$lib/stores';
  import { debounce } from '$lib/utils/debounce';
  import { searchElfEntries, getElfCount } from '$lib/commands';
  import VirtualList from './VirtualList.svelte';

  interface Props {
    oncontextmenu?: (e: CustomEvent<{ x: number; y: number; indices: number[] }>) => void;
  }
  
  let { oncontextmenu }: Props = $props();

  let hoveredIndex = $state<number | null>(null);
  let focusedIndex = $state<number | null>(null);
  let searchQuery = $state($elfSearchQuery);
  let virtualListRef: VirtualList<A2lEntry>;
  let localLoading = $state(false);
  
  let filteredCount = $derived($elfTotalCount);
  
  const ROW_HEIGHT = 32;
  const BUFFER_SIZE = 50;

  // 列宽状态 (百分比)
  let colWidths = $state({ name: 50, type: 25, addr: 25 });
  let resizing = $state<{ col: 'name' | 'type' | null; startX: number; startWidths: typeof colWidths } | null>(null);

  function startResize(col: 'name' | 'type', e: MouseEvent) {
    e.preventDefault();
    resizing = { col, startX: e.clientX, startWidths: { ...colWidths } };
    document.addEventListener('mousemove', handleResize);
    document.addEventListener('mouseup', stopResize);
  }

  function handleResize(e: MouseEvent) {
    if (!resizing) return;
    const container = document.querySelector('.table-header') as HTMLElement;
    if (!container) return;
    const containerWidth = container.offsetWidth;
    const delta = e.clientX - resizing.startX;
    const deltaPercent = (delta / containerWidth) * 100;

    if (resizing.col === 'name') {
      colWidths = {
        name: Math.max(20, Math.min(70, resizing.startWidths.name + deltaPercent)),
        type: Math.max(10, Math.min(40, resizing.startWidths.type - deltaPercent)),
        addr: resizing.startWidths.addr
      };
    } else if (resizing.col === 'type') {
      colWidths = {
        name: resizing.startWidths.name,
        type: Math.max(10, Math.min(40, resizing.startWidths.type + deltaPercent)),
        addr: Math.max(10, Math.min(40, resizing.startWidths.addr - deltaPercent))
      };
    }
  }

  function stopResize() {
    resizing = null;
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
  }

  // 获取第一个排序配置
  function getPrimarySortConfig(configs: SortConfig[]): { field: 'name' | 'address'; order: 'asc' | 'desc' } {
    if (configs.length === 0) {
      return { field: 'name', order: 'asc' };
    }
    const config = configs[0];
    return {
      field: config.field as 'name' | 'address',
      order: config.order
    };
  }

  // 加载数据（带排序参数）
  async function loadData(query: string) {
    localLoading = true;
    try {
      const sortConfig = getPrimarySortConfig($elfSortConfigs);
      const entries = await searchElfEntries(query, 0, 10000, sortConfig.field, sortConfig.order);
      elfEntries.set(entries);
      filteredCount = await getElfCount();
    } catch (e) {
      console.error('加载数据失败:', e);
    }
    localLoading = false;
  }

  // 排序功能
  function handleSort(field: SortField, e: MouseEvent) {
    e.preventDefault();
    const newConfigs = toggleSort($elfSortConfigs, field, e.shiftKey);
    elfSortConfigs.set(newConfigs);
    // 排序变化时重新加载数据
    loadData(searchQuery);
  }

  function getSortIndicator(field: SortField, configs: SortConfig[]): string {
    const index = configs.findIndex(c => c.field === field);
    if (index === -1) return '';
    const config = configs[index];
    const arrow = config.order === 'asc' ? '▲' : '▼';
    const priority = configs.length > 1 ? ` ${index + 1}` : '';
    return `${arrow}${priority}`;
  }

  // 后端已排序，直接使用 elfEntries
  let displayEntries = $derived($elfEntries);

  const debouncedSearch = debounce(async (query: string) => {
    elfSearchQuery.set(query);
    
    if (query || $elfEntries.length === 0) {
      loadData(query);
    }
  }, 300);

  function handleSearch(e: Event) {
    const target = e.target as HTMLInputElement;
    searchQuery = target.value;
    debouncedSearch(target.value);
  }

  function clearSearch() {
    searchQuery = '';
    elfSearchQuery.set('');
    loadData('');
  }

  function handleClick(e: MouseEvent, entry: A2lEntry, displayIndex: number) {
    const displayIndices = displayEntries.map((ent: A2lEntry) => ent.index);
    toggleElfSelection(displayIndex, entry.index, e.ctrlKey, e.shiftKey, displayIndices);
  }

  function handleMouseDown(e: MouseEvent, entry: A2lEntry) {
    if (e.shiftKey) {
      e.preventDefault();
    }
  }

  function handleContextMenu(e: MouseEvent, entry: A2lEntry, displayIndex: number) {
    e.preventDefault();
    const index = entry.index;
    if (!$elfSelectedIndices.has(index)) {
      const displayIndices = displayEntries.map((ent: A2lEntry) => ent.index);
      toggleElfSelection(displayIndex, index, false, false, displayIndices);
    }
    oncontextmenu?.(new CustomEvent('contextmenu', { 
      detail: { x: e.clientX, y: e.clientY, indices: Array.from($elfSelectedIndices) } 
    }));
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'a' && e.ctrlKey) {
      e.preventDefault();
      const allIndices = new Set($elfEntries.map((entry: A2lEntry) => entry.index));
      elfSelectedIndices.set(allIndices);
      return;
    }
    
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const indices = $elfEntries.map((entry: A2lEntry) => entry.index);
      if (indices.length === 0) return;
      
      if (focusedIndex === null) {
        focusedIndex = e.key === 'ArrowDown' ? indices[0] : indices[indices.length - 1];
      } else {
        const currentPos = indices.indexOf(focusedIndex);
        if (currentPos === -1) {
          focusedIndex = e.key === 'ArrowDown' ? indices[0] : indices[indices.length - 1];
        } else {
          const newPos = e.key === 'ArrowDown' 
            ? Math.min(currentPos + 1, indices.length - 1)
            : Math.max(currentPos - 1, 0);
          focusedIndex = indices[newPos];
        }
      }
      const allIndices = $elfEntries.map((ent: A2lEntry) => ent.index);
      const displayPos = allIndices.indexOf(focusedIndex);
      toggleElfSelection(displayPos, focusedIndex, false, false, allIndices);
      
      const idx = indices.indexOf(focusedIndex);
      if (idx !== -1 && virtualListRef) {
        virtualListRef.scrollToIndex(idx);
      }
    }
  }

  function formatAddress(addr: number): string {
    return '0x' + addr.toString(16).toUpperCase().padStart(8, '0');
  }
</script>

<div class="panel" onkeydown={handleKeydown} tabindex="0">
  <div class="header">
    <span class="title">ELF 变量</span>
    <span class="count">({filteredCount.toLocaleString()})</span>
    {#if localLoading || $isLoading}
      <span class="loading">⏳</span>
    {/if}
  </div>
  
  <div class="search-bar">
    <span class="search-icon">🔍</span>
    <input 
      type="text" 
      placeholder="搜索 ELF 变量..." 
      value={searchQuery}
      oninput={handleSearch}
    />
    {#if searchQuery}
      <button class="clear-btn" onclick={clearSearch}>✖</button>
    {/if}
  </div>

  <div class="table-header">
    <span class="col-name sortable" style="width: {colWidths.name}%;" onclick={(e) => handleSort('name', e)}>
      变量名 {getSortIndicator('name', $elfSortConfigs)}
    </span>
    <div class="col-resize" onmousedown={(e) => startResize('name', e)}></div>
    <span class="col-type" style="width: {colWidths.type}%;">类型</span>
    <div class="col-resize" onmousedown={(e) => startResize('type', e)}></div>
    <span class="col-addr sortable" style="width: {colWidths.addr}%;" onclick={(e) => handleSort('address', e)}>
      地址 {getSortIndicator('address', $elfSortConfigs)}
    </span>
  </div>

  <div class="list">
    <VirtualList 
      items={displayEntries} 
      itemHeight={ROW_HEIGHT} 
      bind:this={virtualListRef}
    >
      {#snippet children(entry: A2lEntry, i: number)}
        {@const isSelected = $elfSelectedIndices.has(entry.index)}
        {@const isHovered = hoveredIndex === entry.index}
        {@const isExisting = $a2lNames.has(entry.full_name)}
        
        <div 
          class="row"
          class:selected={isSelected}
          class:hovering={isHovered}
          class:focused={focusedIndex === entry.index}
          data-index={entry.index}
          onclick={(e) => handleClick(e, entry, i)}
          onmousedown={(e) => handleMouseDown(e, entry)}
          oncontextmenu={(e) => handleContextMenu(e, entry, i)}
          onmouseenter={() => hoveredIndex = entry.index}
          onmouseleave={() => hoveredIndex = null}
          role="button"
        >
          <span class="col-name" class:muted={isExisting} style="width: {colWidths.name}%;">{entry.full_name}</span>
          <span class="col-type" style="width: {colWidths.type}%;">{entry.a2l_type}</span>
          <span class="col-addr" style="width: {colWidths.addr}%;">{formatAddress(entry.address)}</span>
        </div>
      {/snippet}
    </VirtualList>
    
    {#if filteredCount > $elfEntries.length}
      <div class="more">显示前 {$elfEntries.length.toLocaleString()} / {filteredCount.toLocaleString()} 个结果</div>
    {/if}
  </div>

  <div class="footer">
    显示: {$elfEntries.length.toLocaleString()} / {filteredCount.toLocaleString()}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
  }

  .header {
    padding: 8px 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .count {
    color: var(--text-muted);
    font-size: 12px;
  }

  .loading {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .search-bar {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    gap: 8px;
  }

  .search-bar input {
    flex: 1;
    padding: 6px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-size: 13px;
  }

  .search-bar input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .clear-btn {
    padding: 4px 8px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .table-header {
    display: flex;
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }

  .table-header .col-name,
  .table-header .col-type,
  .table-header .col-addr {
    text-align: center;
    font-weight: 500;
  }

  .table-header .sortable {
    cursor: pointer;
    user-select: none;
    transition: color 0.2s;
  }

  .table-header .sortable:hover {
    color: var(--accent);
  }

  .list {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .row {
    display: flex;
    padding: 6px 12px;
    cursor: pointer;
    border-left: 2px solid transparent;
    height: 32px;
    box-sizing: border-box;
  }

  .row:hover, .row.hovering {
    background: var(--bg-hover);
  }

  .row.selected {
    background: var(--bg-selected);
    border-left-color: var(--accent);
  }

  .row.focused {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .row.existing .col-name {
    color: var(--text-muted);
  }

  .muted {
    color: var(--text-muted);
  }

  .col-name {
    font-family: monospace;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-type {
    font-family: monospace;
    font-size: 11px;
    color: var(--text-muted);
    text-align: center;
  }

  .col-addr {
    font-family: monospace;
    font-size: 11px;
    color: var(--text-muted);
    text-align: right;
  }

  .col-resize {
    width: 8px;
    cursor: col-resize;
    flex-shrink: 0;
    margin: -6px -2px;
    padding: 6px 2px;
    background: transparent;
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
    transition: all 0.2s;
  }

  .col-resize:hover {
    background: var(--accent);
    border-color: var(--accent);
  }

  .more {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 8px 12px;
    color: var(--text-muted);
    font-size: 12px;
    text-align: center;
    background: var(--bg);
    border-top: 1px solid var(--border);
  }

  .footer {
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text-muted);
    border-top: 1px solid var(--border);
  }
</style>
