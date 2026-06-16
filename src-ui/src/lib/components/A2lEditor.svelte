<script lang="ts">
  import { 
    a2lVariables, a2lSelectedNames, statusMessage, showCompuMethodPanel
  } from '$lib/stores';
  import { saveA2lChanges, searchA2lVariables, listCompuMethods } from '$lib/commands';
  import type { A2lVariable, A2lVariableEdit, CompuMethodSummary } from '$lib/types';

  const A2L_TYPES = ['UBYTE', 'SBYTE', 'UWORD', 'SWORD', 'ULONG', 'SLONG', 'A_UINT64', 'A_INT64', 'FLOAT32_IEEE', 'FLOAT64_IEEE'];

  function getDefaultBitMask(dataType: string): string {
    const masks: Record<string, string> = {
      'UBYTE': '0x00',
      'SBYTE': '0x00',
      'UWORD': '0x0000',
      'SWORD': '0x0000',
      'ULONG': '0x00000000',
      'SLONG': '0x00000000',
      'A_UINT64': '0x0000000000000000',
      'A_INT64': '0x0000000000000000',
      'FLOAT32_IEEE': '0x00000000',
      'FLOAT64_IEEE': '0x0000000000000000',
    };
    return masks[dataType] || '0x00';
  }

  let editBuffer = $state<{
    name: string;
    address: string;
    data_type: string;
    bit_mask: string;
    f: string;
    offset: string;
    unit: string;
  }>({ name: '', address: '', data_type: '', bit_mask: '', f: '', offset: '', unit: '' });

  let originalValues = $state<{
    name: string;
    address: string;
    data_type: string;
    bit_mask: string;
    f: string;
    offset: string;
    unit: string;
  } | null>(null);

  let isSaving = $state(false);
  let compuMethodList = $state<CompuMethodSummary[]>([]);
  let selectedCompuMethod = $state<string>('');

  async function loadCompuMethodList() {
    try {
      compuMethodList = await listCompuMethods();
    } catch {
      compuMethodList = [];
    }
  }

  async function handleCompuMethodChange() {
    if (!selectedVariable || !selectedCompuMethod) return;
    if (selectedCompuMethod === '__none__') {
      selectedCompuMethod = '';
      return;
    }
    const existing = selectedVariable.compu_method || '';
    if (selectedCompuMethod === existing) return;

    isSaving = true;
    statusMessage.set('⏳ 正在关联转换关系...');
    try {
      const change: A2lVariableEdit = {
        action: 'modify',
        originalName: selectedVariable.name,
        compu_method: selectedCompuMethod,
      };
      await saveA2lChanges([change]);
      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);
      statusMessage.set(`✅ 已关联 ${selectedCompuMethod}`);
      selectedCompuMethod = '';
    } catch (e) {
      statusMessage.set(`❌ 关联失败: ${e}`);
      selectedCompuMethod = '';
    }
    isSaving = false;
  }

  let selectedVariable = $derived.by(() => {
    const names = Array.from($a2lSelectedNames);
    if (names.length !== 1) return null;
    return $a2lVariables.find((v: A2lVariable) => v.name === names[0]) || null;
  });

  $effect(() => {
    if (selectedVariable) {
      loadCompuMethodList();
      editBuffer = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        bit_mask: selectedVariable.bit_mask || getDefaultBitMask(selectedVariable.data_type),
        f: selectedVariable.f != null ? String(selectedVariable.f) : '',
        offset: selectedVariable.offset != null ? String(selectedVariable.offset) : '',
        unit: selectedVariable.unit || '',
      };
      originalValues = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        bit_mask: selectedVariable.bit_mask || getDefaultBitMask(selectedVariable.data_type),
        f: selectedVariable.f != null ? String(selectedVariable.f) : '',
        offset: selectedVariable.offset != null ? String(selectedVariable.offset) : '',
        unit: selectedVariable.unit || '',
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
      editBuffer.bit_mask !== originalValues.bit_mask ||
      editBuffer.f !== originalValues.f ||
      editBuffer.offset !== originalValues.offset ||
      editBuffer.unit !== originalValues.unit
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
      if (editBuffer.bit_mask !== originalValues.bit_mask) change.bit_mask = editBuffer.bit_mask;

      const fChanged = editBuffer.f !== originalValues.f;
      const offsetChanged = editBuffer.offset !== originalValues.offset;
      const unitChanged = editBuffer.unit !== originalValues.unit;

      if (fChanged || offsetChanged) {
        const fVal = editBuffer.f ? parseFloat(editBuffer.f) : null;
        const offsetVal = editBuffer.offset ? parseFloat(editBuffer.offset) : null;
        if (fVal !== null && offsetVal !== null) {
          change.f = fVal;
          change.offset = offsetVal;
        }
      }
      if (unitChanged) {
        change.unit = editBuffer.unit || undefined;
      }

      await saveA2lChanges([change]);
      
      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);
      
      const origName = originalValues?.name || '';
      const updatedVar = variables.find((v: A2lVariable) => v.name === (change.name || origName));
      if (updatedVar) {
        originalValues = {
          name: updatedVar.name,
          address: updatedVar.address || '',
          data_type: updatedVar.data_type,
          bit_mask: updatedVar.bit_mask || getDefaultBitMask(updatedVar.data_type),
          f: updatedVar.f != null ? String(updatedVar.f) : '',
          offset: updatedVar.offset != null ? String(updatedVar.offset) : '',
          unit: updatedVar.unit || '',
        };
      } else {
        originalValues = { ...editBuffer };
      }
      
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
        <span class="field-label">BIT_MASK</span>
        <input 
          type="text" 
          bind:value={editBuffer.bit_mask}
          class="field-input"
          placeholder="0x... (可选)"
          disabled={isSaving}
        />
      </label>
    </div>

    <div class="section-divider">
      <span class="section-label">转化系数 (COMPU_METHOD)</span>
      <button class="manage-btn" onclick={() => showCompuMethodPanel.set(true)}>🔧 管理</button>
    </div>

    {#if compuMethodList.length > 0}
      <div class="editor-row">
        <label class="cm-select-label">
          <span class="field-label">关联</span>
          <select
            class="field-select"
            value={selectedCompuMethod}
            onchange={handleCompuMethodChange}
            disabled={isSaving}
          >
            <option value="">-- 选择转换关系 --</option>
            <option value="__none__">NO_COMPU_METHOD</option>
            {#each compuMethodList as cm}
              <option value={cm.name}>{cm.name} ({cm.conversion_type})</option>
            {/each}
          </select>
        </label>
      </div>
    {/if}

    <div class="editor-row">
      <label>
        <span class="field-label">F (斜率)</span>
        <input 
          type="text" 
          bind:value={editBuffer.f}
          class="field-input"
          placeholder="如 0.5, 1.0, 0.01"
          disabled={isSaving}
        />
      </label>
      <label>
        <span class="field-label">OFFSET</span>
        <input 
          type="text" 
          bind:value={editBuffer.offset}
          class="field-input"
          placeholder="如 0, -273.15, 10"
          disabled={isSaving}
        />
      </label>
      <label>
        <span class="field-label">Unit</span>
        <input 
          type="text" 
          bind:value={editBuffer.unit}
          class="field-input"
          placeholder="如 °C, rpm, ms"
          disabled={isSaving}
        />
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

  .section-divider {
    display: flex;
    align-items: center;
    margin: 8px 0 6px 0;
    padding-top: 4px;
    border-top: 1px dashed var(--border);
  }

  .section-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .manage-btn {
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 10px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text-muted);
  }

  .manage-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .cm-select-label {
    width: 100%;
  }
</style>
