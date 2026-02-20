<script lang="ts">
  import { 
    a2lVariables, a2lSearchQuery, a2lSelectedNames, toggleA2lSelection,
    a2lSortConfigs, toggleSort, applySorting, parseAddress
  } from '$lib/stores';
  import type { A2lVariable } from '$lib/types';
  import type { SortField, SortConfig } from '$lib/stores';
  import { debounce } from '$lib/utils/debounce';
  import VirtualList from './VirtualList.svelte';
  import A2lEditor from './A2lEditor.svelte';
  import AddVariableDialog from './AddVariableDialog.svelte';

  interface Props {
    oncontextmenu?: (e: CustomEvent<{ x: number; y: number; names: string[] }>) => void;
  }
  
  let { oncontextmenu }: Props = $props();

  let hoveredName = $state<string | null>(null);
  let focusedName = $state<string | null>(null);
  let searchQuery = $state($a2lSearchQuery);
  let virtualListRef: VirtualList<A2lVariable>;
  let showAddDialog = $state(false);
  
  // 编辑器面板高度 (像素)
  let editorHeight = $state(120);
  let isResizingEditor = $state(false);
  const MIN_EDITOR_HEIGHT = 80;
  const MAX_EDITOR_HEIGHT_RATIO = 0.6;
  
  // 列宽状态 (百分比)
  let colWidths = $state({ icon: 6, name: 44, type: 25, addr: 25 });
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
        icon: resizing.startWidths.icon,
        name: Math.max(20, Math.min(70, resizing.startWidths.name + deltaPercent)),
        type: Math.max(10, Math.min(40, resizing.startWidths.type - deltaPercent)),
        addr: resizing.startWidths.addr
      };
    } else if (resizing.col === 'type') {
      colWidths = {
        icon: resizing.startWidths.icon,
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

  function startResizeEditor(e: MouseEvent) {
    e.preventDefault();
    isResizingEditor = true;
    document.addEventListener('mousemove', handleResizeEditor);
    document.addEventListener('mouseup', stopResizeEditor);
  }

  function handleResizeEditor(e: MouseEvent) {
    const panel = document.querySelector('.panel') as HTMLElement;
    if (!panel) return;
    const rect = panel.getBoundingClientRect();
    const newHeight = rect.bottom - e.clientY;
    const maxHeight = rect.height * MAX_EDITOR_HEIGHT_RATIO;
    editorHeight = Math.max(MIN_EDITOR_HEIGHT, Math.min(maxHeight, newHeight));
  }

  function stopResizeEditor() {
    isResizingEditor = false;
    document.removeEventListener('mousemove', handleResizeEditor);
    document.removeEventListener('mouseup', stopResizeEditor);
  }
  
  // 排序功能
  function handleSort(field: SortField, e: MouseEvent) {
    e.preventDefault();
    const newConfigs = toggleSort($a2lSortConfigs, field, e.shiftKey);
    a2lSortConfigs.set(newConfigs);
  }

  function getSortIndicator(field: SortField, configs: SortConfig[]): string {
    const index = configs.findIndex(c => c.field === field);
    if (index === -1) return '';
    const config = configs[index];
    const arrow = config.order === 'asc' ? '▲' : '▼';
    const priority = configs.length > 1 ? ` ${index + 1}` : '';
    return `${arrow}${priority}`;
  }

  // A2L 变量的排序获取函数
  function getA2lFieldValue(variable: A2lVariable, field: SortField): string | number {
    if (field === 'name') {
      return variable.name;
    } else {
      return parseAddress(variable.address);
    }
  }
  
  let displayVars = $derived.by(() => {
    const query = $a2lSearchQuery.toLowerCase();
    let filtered = $a2lVariables;
    
    if (query) {
      filtered = filtered.filter((v: A2lVariable) => v.name.toLowerCase().includes(query));
    }
    
    // 应用排序
    return applySorting(filtered, $a2lSortConfigs, getA2lFieldValue);
  });

  // 暴露给父组件的方法：滚动到指定变量
  export function scrollToVariable(name: string): boolean {
    const index = displayVars.findIndex(v => v.name === name);
    if (index >= 0 && virtualListRef) {
      virtualListRef.scrollToIndex(index);
      return true;
    }
    return false;
  }

  const ROW_HEIGHT = 32;

  const debouncedSetQuery = debounce((value: string) => {
    a2lSearchQuery.set(value);
  }, 300);

  function handleSearch(e: Event) {
    const target = e.target as HTMLInputElement;
    searchQuery = target.value;
    debouncedSetQuery(target.value);
  }

  function clearSearch() {
    searchQuery = '';
    a2lSearchQuery.set('');
  }

  function handleClick(e: MouseEvent, name: string) {
    const allNames = displayVars.map((v: A2lVariable) => v.name);
    toggleA2lSelection(name, e.ctrlKey, e.shiftKey, allNames);
  }

  function handleMouseDown(e: MouseEvent) {
    if (e.shiftKey) {
      e.preventDefault();
    }
  }

  function handleContextMenu(e: MouseEvent, name: string) {
    e.preventDefault();
    const allNames = displayVars.map((v: A2lVariable) => v.name);
    if (!$a2lSelectedNames.has(name)) {
      toggleA2lSelection(name, false, false, allNames);
    }
    oncontextmenu?.(new CustomEvent('contextmenu', { 
      detail: { x: e.clientX, y: e.clientY, names: Array.from($a2lSelectedNames) } 
    }));
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'a' && e.ctrlKey) {
      e.preventDefault();
      const allNames = new Set(displayVars.map((v: A2lVariable) => v.name));
      a2lSelectedNames.set(allNames);
      return;
    }
    
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const names = displayVars.map((v: A2lVariable) => v.name);
      if (names.length === 0) return;
      
      if (focusedName === null) {
        focusedName = e.key === 'ArrowDown' ? names[0] : names[names.length - 1];
      } else {
        const currentIdx = names.indexOf(focusedName);
        if (currentIdx === -1) {
          focusedName = e.key === 'ArrowDown' ? names[0] : names[names.length - 1];
        } else {
          const newIdx = e.key === 'ArrowDown' 
            ? Math.min(currentIdx + 1, names.length - 1)
            : Math.max(currentIdx - 1, 0);
          focusedName = names[newIdx];
        }
      }
      
      const allNames = displayVars.map((v: A2lVariable) => v.name);
      toggleA2lSelection(focusedName, false, false, allNames);
      
      const idx = names.indexOf(focusedName);
      if (idx !== -1 && virtualListRef) {
        virtualListRef.scrollToIndex(idx);
      }
    }
  }

  function formatAddress(addr: string | null): string {
    if (!addr) return 'N/A';
    return addr;
  }

  function getVarTypeIcon(varType: string): string {
    if (varType === 'CHARACTERISTIC') {
      return `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 256 256"><path fill="#f59e0b" d="M32 80a8 8 0 0 1 8-8h37.17a28 28 0 0 1 53.66 0H216a8 8 0 0 1 0 16h-85.17a28 28 0 0 1-53.66 0H40a8 8 0 0 1-8-8m184 88h-21.17a28 28 0 0 0-53.66 0H40a8 8 0 0 0 0 16h101.17a28 28 0 0 0 53.66 0H216a8 8 0 0 0 0-16"/></svg>`;
    }
    return `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 256 256"><path fill="#3b82f6" d="M240 152v24a16 16 0 0 1-16 16H115.93a4 4 0 0 1-3.24-6.35L174.27 101a8.21 8.21 0 0 0-1.37-11.3a8 8 0 0 0-11.37 1.61l-72 99.06a4 4 0 0 1-3.28 1.63H32a16 16 0 0 1-16-16v-22.87c0-1.79 0-3.57.13-5.33a4 4 0 0 1 4-3.8H48a8 8 0 0 0 8-8.53a8.17 8.17 0 0 0-8.27-7.47H23.92a4 4 0 0 1-3.87-5c12-43.84 49.66-77.13 95.52-82.28a4 4 0 0 1 4.43 4V72a8 8 0 0 0 8.53 8a8.17 8.17 0 0 0 7.47-8.27V44.67a4 4 0 0 1 4.43-4a112.18 112.18 0 0 1 95.8 82.33a4 4 0 0 1-3.88 5h-24.08a8.17 8.17 0 0 0-8.25 7.47a8 8 0 0 0 8 8.53h27.92a4 4 0 0 1 4 3.86c.06 1.37.06 2.75.06 4.14"/></svg>`;
  }

  function getVarTypeLabel(varType: string): string {
    return varType === 'CHARACTERISTIC' ? '标定' : '观测';
  }
</script>

<div class="panel" onkeydown={handleKeydown} tabindex="0">
  <div class="header">
    <span class="title">A2L 变量</span>
    <span class="count">({$a2lVariables.length.toLocaleString()})</span>
  </div>
  
  <div class="search-bar">
    <span class="search-icon">🔍</span>
    <input 
      type="text" 
      placeholder="搜索 A2L 变量..." 
      value={searchQuery}
      oninput={handleSearch}
    />
    {#if searchQuery}
      <button class="clear-btn" onclick={clearSearch}>✖</button>
    {/if}
    <button class="add-btn" onclick={() => showAddDialog = true} title="手动添加变量">➕</button>
  </div>

  <div class="table-header">
    <span class="col-icon" style="width: {colWidths.icon}%;"> </span>
    <span class="col-name sortable" style="width: {colWidths.name}%;" onclick={(e) => handleSort('name', e)}>
      变量名 {getSortIndicator('name', $a2lSortConfigs)}
    </span>
    <div class="col-resize" onmousedown={(e) => startResize('name', e)}></div>
    <span class="col-type" style="width: {colWidths.type}%;">类型</span>
    <div class="col-resize" onmousedown={(e) => startResize('type', e)}></div>
    <span class="col-addr sortable" style="width: {colWidths.addr}%;" onclick={(e) => handleSort('address', e)}>
      地址 {getSortIndicator('address', $a2lSortConfigs)}
    </span>
  </div>

  <div class="list">
    <VirtualList 
      items={displayVars} 
      itemHeight={ROW_HEIGHT} 
      bind:this={virtualListRef}
    >
      {#snippet children(variable: A2lVariable, i: number)}
        {@const isSelected = $a2lSelectedNames.has(variable.name)}
        {@const isHovered = hoveredName === variable.name}
        
        <div 
          class="row"
          class:selected={isSelected}
          class:hovering={isHovered}
          class:focused={focusedName === variable.name}
          class:characteristic={variable.var_type === 'CHARACTERISTIC'}
          data-index={i}
          onclick={(e) => handleClick(e, variable.name)}
          onmousedown={handleMouseDown}
          oncontextmenu={(e) => handleContextMenu(e, variable.name)}
          onmouseenter={() => hoveredName = variable.name}
          onmouseleave={() => hoveredName = null}
          role="button"
        >
          <span class="col-icon" style="width: {colWidths.icon}%;" title={getVarTypeLabel(variable.var_type)}>{@html getVarTypeIcon(variable.var_type)}</span>
          <span class="col-name" style="width: {colWidths.name}%;">{variable.name}</span>
          <span class="col-type" style="width: {colWidths.type}%;">{variable.data_type}</span>
          <span class="col-addr" style="width: {colWidths.addr}%;">{formatAddress(variable.address)}</span>
        </div>
      {/snippet}
    </VirtualList>
  </div>

  <div class="footer">
    显示: {displayVars.length.toLocaleString()}
  </div>
  
  <div 
    class="editor-resizer" 
    class:active={isResizingEditor}
    onmousedown={startResizeEditor}
  ></div>
  
  <div class="editor-container" style="height: {editorHeight}px;">
    <A2lEditor />
  </div>
</div>

<AddVariableDialog visible={showAddDialog} onclose={() => showAddDialog = false} />

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
    flex-shrink: 0;
  }

  .count {
    color: var(--text-muted);
    font-size: 12px;
  }

  .search-bar {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    gap: 8px;
    flex-shrink: 0;
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

  .add-btn {
    padding: 4px 10px;
    background: var(--accent);
    border: none;
    border-radius: 4px;
    color: white;
    cursor: pointer;
    font-size: 12px;
    transition: opacity 0.2s;
  }

  .add-btn:hover {
    opacity: 0.85;
  }

  .table-header {
    display: flex;
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .table-header .col-name,
  .table-header .col-type,
  .table-header .col-addr,
  .table-header .col-icon {
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
    min-height: 0;
    overflow: hidden;
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

  .col-icon {
    font-size: 12px;
    text-align: center;
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

  .footer {
    padding: 6px 12px;
    font-size: 11px;
    color: var(--text-muted);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .editor-resizer {
    height: 6px;
    background: var(--border);
    cursor: row-resize;
    flex-shrink: 0;
    transition: background 0.2s;
  }

  .editor-resizer:hover,
  .editor-resizer.active {
    background: var(--accent);
  }

  .editor-container {
    flex-shrink: 0;
    overflow: hidden;
  }
</style>
