<script lang="ts">
  import { 
    a2lVariables, a2lSelectedNames, statusMessage
  } from '$lib/stores';
  import { saveA2lChanges, searchA2lVariables } from '$lib/commands';
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

  let isSaving = $state(false);

  let selectedVariable = $derived.by(() => {
    const names = Array.from($a2lSelectedNames);
    if (names.length !== 1) return null;
    return $a2lVariables.find((v: A2lVariable) => v.name === names[0]) || null;
  });

  // 当选中变量变化时，更新编辑缓冲区
  $effect(() => {
    if (selectedVariable) {
      editBuffer = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        var_type: selectedVariable.var_type,
      };
      originalValues = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        var_type: selectedVariable.var_type,
      };
    } else {
      originalValues = null;
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

  async function handleSave() {
    if (!originalValues || !hasChanges || isSaving) return;
    
    isSaving = true;
    statusMessage.set('⏳ 正在保存...');
    
    try {
      const change: A2lVariableEdit = {
        action: 'modify',
        originalName: originalValues.name,
      };

      if (editBuffer.name !== originalValues.name) change.name = editBuffer.name;
      if (editBuffer.address !== originalValues.address) change.address = editBuffer.address;
      if (editBuffer.data_type !== originalValues.data_type) change.data_type = editBuffer.data_type;
      if (editBuffer.var_type !== originalValues.var_type) change.var_type = editBuffer.var_type;

      await saveA2lChanges([change]);
      
      // 刷新列表
      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);
      
      // 更新 originalValues 为新值
      originalValues = { ...editBuffer };
      
      statusMessage.set('✅ 已保存');
    } catch (e) {
      statusMessage.set(`❌ 保存失败: ${e}`);
    }
    
    isSaving = false;
  }
</script>

{#if selectedVariable && originalValues}
  <div class="editor">
    <div class="editor-header">
      <span class="label">编辑:</span>
      <span class="var-name">{originalValues.name}</span>
    </div>
    
    <div class="editor-row">
      <label>
        <span class="field-label">名称</span>
        <input 
          type="text" 
          bind:value={editBuffer.name}
          class="field-input"
          disabled={isSaving}
        />
      </label>
      <label>
        <span class="field-label">地址</span>
        <input 
          type="text" 
          bind:value={editBuffer.address}
          class="field-input"
          placeholder="0x..."
          disabled={isSaving}
        />
      </label>
    </div>
    
    <div class="editor-row">
      <label>
        <span class="field-label">数据类型</span>
        <select bind:value={editBuffer.data_type} class="field-select" disabled={isSaving}>
          {#each A2L_TYPES as t}
            <option value={t}>{t}</option>
          {/each}
        </select>
      </label>
      <label>
        <span class="field-label">变量类型</span>
        <select bind:value={editBuffer.var_type} class="field-select" disabled={isSaving}>
          {#each VAR_TYPES as t}
            <option value={t}>{t === 'MEASUREMENT' ? '观测' : '标定'}</option>
          {/each}
        </select>
      </label>
    </div>
    
    <div class="editor-actions">
      <button 
        class="btn btn-primary" 
        onclick={handleSave}
        disabled={!hasChanges || isSaving}
      >
        {#if isSaving}
          保存中...
        {:else}
          💾 保存
        {/if}
      </button>
    </div>
  </div>
{:else if $a2lSelectedNames.size > 1}
  <div class="editor placeholder">
    <span class="placeholder-text">已选中 {$a2lSelectedNames.size} 个变量</span>
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

  .field-input:disabled, .field-select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
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

  .btn-primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
    background: var(--accent);
  }
</style>
