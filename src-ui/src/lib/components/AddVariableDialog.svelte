<script lang="ts">
  import { fly } from 'svelte/transition';
  import { a2lVariables, a2lNames, statusMessage } from '$lib/stores';
  import { saveA2lChanges, searchA2lVariables } from '$lib/commands';
  import type { A2lVariableEdit } from '$lib/types';

  interface Props {
    visible: boolean;
    onclose?: (e: CustomEvent) => void;
  }

  let { visible, onclose }: Props = $props();

  const A2L_TYPES = ['UBYTE', 'SBYTE', 'UWORD', 'SWORD', 'ULONG', 'SLONG', 'A_UINT64', 'A_INT64', 'FLOAT32_IEEE', 'FLOAT64_IEEE'];

  let varType = $state<'MEASUREMENT' | 'CHARACTERISTIC'>('MEASUREMENT');
  let name = $state('');
  let address = $state('');
  let dataType = $state('UBYTE');
  let bitMask = $state('');
  let isSaving = $state(false);
  let nameError = $state('');

  function getDefaultBitMask(type: string): string {
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
    return masks[type] || '0x00';
  }

  $effect(() => {
    if (!bitMask) {
      bitMask = getDefaultBitMask(dataType);
    }
  });

  function validateName(value: string): boolean {
    if (!value.trim()) {
      nameError = '变量名不能为空';
      return false;
    }
    if ($a2lNames.has(value.trim())) {
      nameError = '变量名已存在';
      return false;
    }
    nameError = '';
    return true;
  }

  function validateAddress(value: string): boolean {
    if (!value.trim()) {
      return false;
    }
    const cleaned = value.trim().replace(/^0x/i, '');
    return /^[0-9a-fA-F]+$/.test(cleaned);
  }

  let isValid = $derived(
    name.trim() !== '' &&
    !nameError &&
    validateAddress(address) &&
    !isSaving
  );

  function close() {
    onclose?.(new CustomEvent('close'));
  }

  function resetForm() {
    varType = 'MEASUREMENT';
    name = '';
    address = '';
    dataType = 'UBYTE';
    bitMask = getDefaultBitMask('UBYTE');
    nameError = '';
  }

  async function handleAdd() {
    if (!isValid || isSaving) return;

    isSaving = true;
    statusMessage.set('⏳ 正在添加变量...');

    try {
      const addrValue = address.trim().startsWith('0x') 
        ? address.trim() 
        : `0x${address.trim()}`;

      const edit: A2lVariableEdit = {
        action: 'add',
        originalName: '',
        var_type: varType,
        entry: {
          index: 0,
          full_name: name.trim(),
          address: parseInt(addrValue.replace(/^0x/i, ''), 16),
          size: 0,
          a2l_type: dataType,
          type_name: dataType,
          bit_offset: null,
          bit_size: null,
        },
        exportMode: varType === 'MEASUREMENT' ? 'measurement' : 'characteristic',
      };

      await saveA2lChanges([edit]);

      const variables = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(variables);

      statusMessage.set(`✅ 已添加变量: ${name.trim()}`);
      resetForm();
      close();
    } catch (e) {
      statusMessage.set(`❌ 添加失败: ${e}`);
    }

    isSaving = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
    if (e.key === 'Enter' && isValid) handleAdd();
  }

  function handleOverlayClick() {
    if (!isSaving) close();
  }

  $effect(() => {
    if (visible) {
      resetForm();
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if visible}
  <div class="overlay" onclick={handleOverlayClick} role="dialog" aria-modal="true">
    <div class="dialog" transition:fly={{ duration: 150, y: -20 }} onclick={(e) => e.stopPropagation()}>
      <div class="header">
        <h3>添加 A2L 变量</h3>
        <button class="close-btn" onclick={close} disabled={isSaving}>✖</button>
      </div>

      <div class="content">
        <div class="field-group">
          <label class="field-label">变量类型</label>
          <div class="radio-group">
            <label class="radio-item">
              <input type="radio" name="varType" value="MEASUREMENT" bind:group={varType} disabled={isSaving} />
              <span class="radio-label">观测 (MEASUREMENT)</span>
            </label>
            <label class="radio-item">
              <input type="radio" name="varType" value="CHARACTERISTIC" bind:group={varType} disabled={isSaving} />
              <span class="radio-label">标定 (CHARACTERISTIC)</span>
            </label>
          </div>
        </div>

        <div class="field-group">
          <label class="field-label">变量名称 <span class="required">*</span></label>
          <input
            type="text"
            class="field-input"
            class:error={nameError}
            bind:value={name}
            placeholder="例如: EngineSpeed"
            disabled={isSaving}
            oninput={() => validateName(name)}
          />
          {#if nameError}
            <span class="error-text">{nameError}</span>
          {/if}
        </div>

        <div class="field-group">
          <label class="field-label">地址 (十六进制) <span class="required">*</span></label>
          <input
            type="text"
            class="field-input"
            bind:value={address}
            placeholder="例如: 0x20000000"
            disabled={isSaving}
          />
        </div>

        <div class="field-row">
          <div class="field-group flex-1">
            <label class="field-label">数据类型</label>
            <select class="field-select" bind:value={dataType} disabled={isSaving}>
              {#each A2L_TYPES as t}
                <option value={t}>{t}</option>
              {/each}
            </select>
          </div>
          <div class="field-group flex-1">
            <label class="field-label">BIT_MASK (可选)</label>
            <input
              type="text"
              class="field-input"
              bind:value={bitMask}
              placeholder="0x..."
              disabled={isSaving}
            />
          </div>
        </div>
      </div>

      <div class="footer">
        <button class="btn secondary" onclick={close} disabled={isSaving}>取消</button>
        <button class="btn primary" onclick={handleAdd} disabled={!isValid}>
          {#if isSaving}
            添加中...
          {:else}
            添加变量
          {/if}
        </button>
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
    min-width: 400px;
    max-width: 500px;
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

  .close-btn:hover:not(:disabled) {
    color: var(--text);
  }

  .close-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .content {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-row {
    display: flex;
    gap: 12px;
  }

  .flex-1 {
    flex: 1;
  }

  .field-label {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 500;
  }

  .required {
    color: #ef4444;
  }

  .field-input, .field-select {
    padding: 8px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-size: 13px;
    font-family: monospace;
  }

  .field-input:focus, .field-select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .field-input.error {
    border-color: #ef4444;
  }

  .field-input:disabled, .field-select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .field-select {
    cursor: pointer;
  }

  .error-text {
    font-size: 11px;
    color: #ef4444;
  }

  .radio-group {
    display: flex;
    gap: 16px;
  }

  .radio-item {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  .radio-item input {
    cursor: pointer;
  }

  .radio-label {
    font-size: 13px;
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .btn.primary {
    background: var(--accent);
    color: white;
  }

  .btn.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.secondary {
    background: var(--bg-hover);
    color: var(--text);
  }

  .btn.secondary:hover:not(:disabled) {
    background: var(--border);
  }

  .btn.secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
