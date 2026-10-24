<script lang="ts">
  import {
    a2lVariables, a2lSelectedNames, statusMessage
  } from '$lib/stores';
  import { saveA2lChanges, searchA2lVariables } from '$lib/commands';
  import type { A2lVariable, A2lVariableEdit } from '$lib/types';

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

  /** 当前变量的线性转换状态快照；nonLinear 表示挂着文字表/插值表（保存时不破坏） */
  interface ConversionState {
    on: boolean;
    f: string;
    offset: string;
    unit: string;
    nonLinear: 'TAB_VERB' | 'TAB_INTP' | null;
    pairCount: number | null;
  }

  function conversionStateFrom(v: A2lVariable): ConversionState {
    const isLinear = v.compu_type === 'LINEAR' && v.f != null && v.offset != null;
    const nonLinear = v.compu_type === 'TAB_VERB' || v.compu_type === 'TAB_INTP'
      ? v.compu_type
      : null;
    return {
      on: isLinear,
      f: isLinear && v.f != null ? String(v.f) : '',
      offset: isLinear && v.offset != null ? String(v.offset) : '',
      unit: isLinear ? (v.unit || '') : '',
      nonLinear,
      pairCount: v.compu_pair_count,
    };
  }

  let editBuffer = $state<{
    name: string;
    address: string;
    data_type: string;
    bit_mask: string;
    conversionOn: boolean;
    f: string;
    offset: string;
    unit: string;
  }>({ name: '', address: '', data_type: '', bit_mask: '', conversionOn: false, f: '', offset: '', unit: '' });

  let originalValues = $state<{
    name: string;
    address: string;
    data_type: string;
    bit_mask: string;
  } | null>(null);

  let originalConversion = $state<ConversionState | null>(null);

  let isSaving = $state(false);

  function handleConversionToggle() {
    editBuffer.conversionOn = !editBuffer.conversionOn;
    if (editBuffer.conversionOn) {
      if (!editBuffer.f) editBuffer.f = '1';
      if (!editBuffer.offset) editBuffer.offset = '0';
    }
  }

  let selectedVariable = $derived.by(() => {
    const names = Array.from($a2lSelectedNames);
    if (names.length !== 1) return null;
    return $a2lVariables.find((v: A2lVariable) => v.name === names[0]) || null;
  });

  $effect(() => {
    if (selectedVariable) {
      const conv = conversionStateFrom(selectedVariable);
      editBuffer = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        bit_mask: selectedVariable.bit_mask || getDefaultBitMask(selectedVariable.data_type),
        conversionOn: conv.on,
        f: conv.f,
        offset: conv.offset,
        unit: conv.unit,
      };
      originalValues = {
        name: selectedVariable.name,
        address: selectedVariable.address || '',
        data_type: selectedVariable.data_type,
        bit_mask: selectedVariable.bit_mask || getDefaultBitMask(selectedVariable.data_type),
      };
      originalConversion = conv;
    } else {
      originalValues = null;
      originalConversion = null;
    }
  });

  let conversionChanged = $derived(
    originalConversion && (
      editBuffer.conversionOn !== originalConversion.on ||
      (editBuffer.conversionOn && (
        editBuffer.f !== originalConversion.f ||
        editBuffer.offset !== originalConversion.offset ||
        editBuffer.unit !== originalConversion.unit
      ))
    )
  );

  let hasChanges = $derived(
    originalValues && (
      editBuffer.name !== originalValues.name ||
      editBuffer.address !== originalValues.address ||
      editBuffer.data_type !== originalValues.data_type ||
      editBuffer.bit_mask !== originalValues.bit_mask ||
      conversionChanged
    )
  );

  /** 把转换开关/输入框的状态写入 change；返回 false 表示校验失败 */
  function buildConversionChange(change: A2lVariableEdit): boolean {
    if (!conversionChanged || !originalConversion) return true;

    if (editBuffer.conversionOn) {
      const fVal = parseFloat(editBuffer.f);
      const offsetVal = parseFloat(editBuffer.offset);
      if (!isFinite(fVal) || !isFinite(offsetVal)) {
        statusMessage.set('❌ F 和 OFFSET 必须填写有效数字');
        return false;
      }
      change.f = fVal;
      change.offset = offsetVal;
      change.unit = editBuffer.unit;
    } else if (originalConversion.on) {
      // 原本是线性转换，关闭 = 明确取消；文字表/插值表/IDENTICAL 保持不动
      change.compu_method = 'NO_COMPU_METHOD';
    }
    return true;
  }

  async function handleSave() {
    if (!originalValues || !hasChanges || isSaving) return;

    const change: A2lVariableEdit = {
      action: 'modify',
      originalName: originalValues.name,
    };

    if (!buildConversionChange(change)) return;

    if (editBuffer.name !== originalValues.name) change.name = editBuffer.name;
    if (editBuffer.address !== originalValues.address) change.address = editBuffer.address;
    if (editBuffer.data_type !== originalValues.data_type) change.data_type = editBuffer.data_type;
    if (editBuffer.bit_mask !== originalValues.bit_mask) change.bit_mask = editBuffer.bit_mask;

    isSaving = true;
    statusMessage.set('⏳ 正在保存...');

    try {
      await saveA2lChanges([change]);

      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);

      const origName = originalValues?.name || '';
      const updatedVar = variables.find((v: A2lVariable) => v.name === (change.name || origName));
      if (updatedVar) {
        const conv = conversionStateFrom(updatedVar);
        originalValues = {
          name: updatedVar.name,
          address: updatedVar.address || '',
          data_type: updatedVar.data_type,
          bit_mask: updatedVar.bit_mask || getDefaultBitMask(updatedVar.data_type),
        };
        originalConversion = conv;
      } else {
        originalValues = {
          name: editBuffer.name,
          address: editBuffer.address,
          data_type: editBuffer.data_type,
          bit_mask: editBuffer.bit_mask,
        };
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
      <span class="section-label">线性转换 (COMPU_METHOD)</span>
    </div>

    <div class="conversion-toggle-row">
      <label class="toggle-label">
        <input
          type="checkbox"
          class="toggle-checkbox"
          checked={editBuffer.conversionOn}
          onchange={handleConversionToggle}
          disabled={isSaving}
        />
        <span>启用转换</span>
      </label>
      {#if originalConversion?.nonLinear}
        <span class="nonlinear-hint">
          当前为{originalConversion.nonLinear === 'TAB_VERB' ? '文字表' : '插值表'}转换{originalConversion.pairCount ? `（${originalConversion.pairCount} 项）` : ''}，保持关闭保存不会改动它；开启并保存将替换为线性转换
        </span>
      {/if}
    </div>

    <div class="editor-row" class:disabled-row={!editBuffer.conversionOn}>
      <label>
        <span class="field-label">F (斜率)</span>
        <input
          type="text"
          bind:value={editBuffer.f}
          class="field-input"
          placeholder="如 0.5, 1.0, 0.01"
          disabled={isSaving || !editBuffer.conversionOn}
        />
      </label>
      <label>
        <span class="field-label">OFFSET</span>
        <input
          type="text"
          bind:value={editBuffer.offset}
          class="field-input"
          placeholder="如 0, -273.15, 10"
          disabled={isSaving || !editBuffer.conversionOn}
        />
      </label>
      <label>
        <span class="field-label">Unit</span>
        <input
          type="text"
          bind:value={editBuffer.unit}
          class="field-input"
          placeholder="如 °C, rpm, ms"
          disabled={isSaving || !editBuffer.conversionOn}
        />
      </label>
    </div>
    {#if !editBuffer.conversionOn && !originalConversion?.nonLinear}
      <p class="hint-text">未启用转换，物理值 = 原始值</p>
    {/if}
    
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

  .conversion-toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    cursor: pointer;
    user-select: none;
  }

  .toggle-checkbox {
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }

  .nonlinear-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .hint-text {
    font-size: 11px;
    color: var(--text-muted);
    margin: 0 0 6px 0;
    opacity: 0.8;
  }

  .editor-row.disabled-row {
    opacity: 0.45;
  }
</style>
