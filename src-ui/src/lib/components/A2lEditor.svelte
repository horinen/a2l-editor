<script lang="ts">
  import { untrack } from 'svelte';
  import { 
    a2lVariables, a2lSelectedIndices, addPendingChange, pendingChanges, removePendingChange
  } from '$lib/stores';
  import type { A2lVariable, A2lVariableEdit } from '$lib/types';

  const A2L_TYPES = ['UBYTE', 'SBYTE', 'UWORD', 'SWORD', 'ULONG', 'SLONG', 'A_UINT64', 'A_INT64', 'FLOAT32_IEEE', 'FLOAT64_IEEE'];
  const VAR_TYPES: ('MEASUREMENT' | 'CHARACTERISTIC')[] = ['MEASUREMENT', 'CHARACTERISTIC'];

  let editBuffer = $state<{
    name: string;
    address: string;
    data_type: string;
    var_type: 'MEASUREMENT' | 'CHARACTERISTIC';
  }>({ name: '', address: '', data_type: '', var_type: 'MEASUREMENT' });

  let originalValues = $state<{
    name: string;
    address: string;
    data_type: string;
    var_type: 'MEASUREMENT' | 'CHARACTERISTIC';
  } | null>(null);

  let hasPendingChange = $state(false);

  let selectedVariable = $derived.by(() => {
    const indices = Array.from($a2lSelectedIndices);
    if (indices.length !== 1) return null;
    return $a2lVariables[indices[0]] || null;
  });

  // 当选中变量变化时，更新编辑缓冲区
  $effect(() => {
    if (selectedVariable) {
      // 使用 untrack 避免追踪 pendingChanges，防止编辑时循环重置
      const pendingChange = untrack(() => 
        $pendingChanges.find(c => c.originalName === selectedVariable.name && c.action === 'modify')
      );
      
      if (pendingChange) {
        editBuffer = {
          name: pendingChange.name || selectedVariable.name,
          address: pendingChange.address || selectedVariable.address || '',
          data_type: pendingChange.data_type || selectedVariable.data_type,
          var_type: pendingChange.var_type || selectedVariable.var_type,
        };
        hasPendingChange = true;
      } else {
        editBuffer = {
          name: selectedVariable.name,
          address: selectedVariable.address || '',
          data_type: selectedVariable.data_type,
          var_type: selectedVariable.var_type,
        };
        hasPendingChange = false;
      }
      originalValues = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        var_type: selectedVariable.var_type,
      };
    } else {
      originalValues = null;
      hasPendingChange = false;
    }
  });

  let hasChanges = $derived(
    originalValues && (
      editBuffer.name !== originalValues.name ||
      editBuffer.address !== originalValues.address ||
      editBuffer.data_type !== originalValues.data_type ||
      editBuffer.var_type !== originalValues.var_type
    )
  );

  // 当 editBuffer 变化时，更新 pendingChanges
  // 使用 $effect.preactive 来避免循环
  $effect.pre(() => {
    if (!originalValues) return;
    
    const { name, address, data_type, var_type } = editBuffer;
    
    if (name !== originalValues.name ||
        address !== originalValues.address ||
        data_type !== originalValues.data_type ||
        var_type !== originalValues.var_type) {
      const change: A2lVariableEdit = {
        action: 'modify',
        originalName: originalValues.name,
      };

      if (name !== originalValues.name) change.name = name;
      if (address !== originalValues.address) change.address = address;
      if (data_type !== originalValues.data_type) change.data_type = data_type;
      if (var_type !== originalValues.var_type) change.var_type = var_type;

      addPendingChange(change);
      hasPendingChange = true;
    } else {
      removePendingChange(originalValues.name, 'modify');
      hasPendingChange = false;
    }
  });

  function resetChanges() {
    if (!originalValues) return;
    removePendingChange(originalValues.name, 'modify');
    editBuffer = { ...originalValues };
    hasPendingChange = false;
  }
</script>

{#if selectedVariable && originalValues}
  <div class="editor">
    <div class="editor-header">
      <span class="label">编辑:</span>
      <span class="var-name">{originalValues.name}</span>
      {#if hasPendingChange}
        <span class="pending-badge" title="有未保存的修改">●</span>
      {/if}
    </div>
    
    <div class="editor-row">
      <label>
        <span class="field-label">名称</span>
        <input 
          type="text" 
          bind:value={editBuffer.name}
          class="field-input"
        />
      </label>
      <label>
        <span class="field-label">地址</span>
        <input 
          type="text" 
          bind:value={editBuffer.address}
          class="field-input"
          placeholder="0x..."
        />
      </label>
    </div>
    
    <div class="editor-row">
      <label>
        <span class="field-label">数据类型</span>
        <select bind:value={editBuffer.data_type} class="field-select">
          {#each A2L_TYPES as t}
            <option value={t}>{t}</option>
          {/each}
        </select>
      </label>
      <label>
        <span class="field-label">变量类型</span>
        <select bind:value={editBuffer.var_type} class="field-select">
          {#each VAR_TYPES as t}
            <option value={t}>{t === 'MEASUREMENT' ? '观测' : '标定'}</option>
          {/each}
        </select>
      </label>
    </div>
    
    <div class="editor-actions">
      <button 
        class="btn btn-secondary" 
        onclick={resetChanges}
        disabled={!hasChanges}
      >
        重置
      </button>
    </div>
  </div>
{:else if $a2lSelectedIndices.size > 1}
  <div class="editor placeholder">
    <span class="placeholder-text">已选中 {$a2lSelectedIndices.size} 个变量</span>
    <span class="placeholder-hint">请选择单个变量进行编辑</span>
  </div>
{:else}
  <div class="editor placeholder">
    <span class="placeholder-text">未选中变量</span>
    <span class="placeholder-hint">从上方列表选择一个变量进行编辑</span>
  </div>
{/if}

<style>
  .editor {
    padding: 8px 12px;
    background: var(--bg);
    border-top: 1px solid var(--border);
  }

  .editor.placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 80px;
    color: var(--text-muted);
  }

  .placeholder-text {
    font-size: 13px;
  }

  .placeholder-hint {
    font-size: 11px;
    margin-top: 4px;
    opacity: 0.7;
  }

  .editor-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
  }

  .editor-header .label {
    font-size: 11px;
    color: var(--text-muted);
  }

  .editor-header .var-name {
    font-family: monospace;
    font-size: 12px;
    font-weight: 500;
  }

  .pending-badge {
    color: #f59e0b;
    font-size: 10px;
  }

  .editor-row {
    display: flex;
    gap: 12px;
    margin-bottom: 6px;
  }

  .editor-row label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .field-label {
    font-size: 11px;
    color: var(--text-muted);
    min-width: 50px;
  }

  .field-input, .field-select {
    flex: 1;
    padding: 4px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-size: 12px;
    font-family: monospace;
  }

  .field-input:focus, .field-select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .field-select {
    cursor: pointer;
  }

  .editor-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .btn {
    padding: 4px 12px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
  }

  .btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: transparent;
  }
</style>
