<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { showCompuMethodPanel, statusMessage, a2lVariables } from '$lib/stores';
  import {
    listCompuMethods,
    getCompuMethod,
    saveCompuMethodCmd,
    deleteCompuMethodCmd,
    previewCompuMethodCmd,
    searchA2lVariables,
  } from '$lib/commands';
  import type {
    CompuMethodSummary,
    CompuMethodDetail,
    CompuMethodInput,
    CompuMethodType,
    PreviewResult,
    TabVerbPair,
    TabIntpPair,
  } from '$lib/types';

  let summaries = $state<CompuMethodSummary[]>([]);
  let selectedName = $state<string | null>(null);
  let detail = $state<CompuMethodDetail | null>(null);
  let isLoading = $state(false);
  let isSaving = $state(false);
  let searchQuery = $state('');
  let isCreatingNew = $state(false);

  let editBuffer = $state<CompuMethodInput>({
    name: '',
    conversion_type: 'LINEAR',
    unit: '',
    description: '',
    f: 1,
    offset: 0,
    verb_pairs: [],
    default_value: '',
    intp_pairs: [],
  });

  let hasChanges = $state(false);
  let originalBuffer = $state<string>('');

  let previewRawStart = $state('0');
  let previewRawEnd = $state('100');
  let previewStep = $state('10');
  let previewResults = $state<PreviewResult[]>([]);

  const TYPE_LABELS: Record<string, string> = {
    LINEAR: '线性',
    TAB_VERB: '文字表',
    TAB_INTP: '插值表',
    IDENTICAL: '无转换',
  };

  const TYPE_COLORS: Record<string, string> = {
    LINEAR: '#4CAF50',
    TAB_VERB: '#FF9800',
    TAB_INTP: '#2196F3',
    IDENTICAL: '#9E9E9E',
  };

  let filteredSummaries = $derived(
    searchQuery.trim() === ''
      ? summaries
      : summaries.filter((s) => s.name.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  async function loadSummaries() {
    isLoading = true;
    try {
      summaries = await listCompuMethods();
    } catch (e) {
      statusMessage.set(`❌ 加载转换关系失败: ${e}`);
    }
    isLoading = false;
  }

  async function selectMethod(name: string) {
    selectedName = name;
    isCreatingNew = false;
    isLoading = true;
    try {
      detail = await getCompuMethod(name);
      editBuffer = {
        name: detail.name,
        conversion_type: detail.conversion_type,
        unit: detail.unit,
        description: detail.description,
        f: detail.f,
        offset: detail.offset,
        verb_pairs: detail.verb_pairs.map((p) => ({ ...p })),
        default_value: detail.default_value,
        intp_pairs: detail.intp_pairs.map((p) => ({ ...p })),
      };
      originalBuffer = JSON.stringify(editBuffer);
      hasChanges = false;
      previewResults = [];
    } catch (e) {
      statusMessage.set(`❌ 加载详情失败: ${e}`);
    }
    isLoading = false;
  }

  function startCreateNew() {
    isCreatingNew = true;
    selectedName = null;
    detail = null;
    editBuffer = {
      name: '',
      conversion_type: 'LINEAR',
      unit: '',
      description: '',
      f: 1,
      offset: 0,
      verb_pairs: [{ in_val: 0, verbal: '' }],
      default_value: '',
      intp_pairs: [
        { in_val: 0, out_val: 0 },
        { in_val: 100, out_val: 100 },
      ],
    };
    originalBuffer = JSON.stringify(editBuffer);
    hasChanges = false;
    previewResults = [];
  }

  $effect(() => {
    hasChanges = JSON.stringify(editBuffer) !== originalBuffer;
  });

  async function handleSave() {
    if (!editBuffer.name.trim()) {
      statusMessage.set('❌ 请输入转换关系名称');
      return;
    }
    isSaving = true;
    try {
      await saveCompuMethodCmd(editBuffer);
      statusMessage.set(`✅ 已保存转换关系: ${editBuffer.name}`);
      await loadSummaries();
      if (isCreatingNew) {
        await selectMethod(editBuffer.name);
      }
      const vars = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(vars);
    } catch (e) {
      statusMessage.set(`❌ 保存失败: ${e}`);
    }
    isSaving = false;
  }

  async function handleDelete() {
    if (!selectedName) return;
    const name = selectedName;
    const refCount = summaries.find((s) => s.name === name)?.ref_count || 0;
    const msg =
      refCount > 0
        ? `⚠️ "${name}" 被 ${refCount} 个变量引用，删除后这些变量将显示 NO_COMPU_METHOD。确认删除？`
        : `确认删除转换关系 "${name}"？`;
    if (!confirm(msg)) return;

    isSaving = true;
    try {
      const count = await deleteCompuMethodCmd(name);
      statusMessage.set(
        count > 0
          ? `✅ 已删除 "${name}"（原引用 ${count} 个变量）`
          : `✅ 已删除 "${name}"`
      );
      selectedName = null;
      detail = null;
      await loadSummaries();
      const vars = await searchA2lVariables('', 0, 10000);
      a2lVariables.set(vars);
    } catch (e) {
      statusMessage.set(`❌ 删除失败: ${e}`);
    }
    isSaving = false;
  }

  async function handlePreview() {
    const start = parseFloat(previewRawStart);
    const end = parseFloat(previewRawEnd);
    const step = parseFloat(previewStep);
    if (isNaN(start) || isNaN(end) || isNaN(step) || step <= 0) {
      statusMessage.set('❌ 预览参数无效');
      return;
    }
    const values: number[] = [];
    if (start <= end) {
      for (let v = start; v <= end + 1e-9; v += step) {
        values.push(Math.round(v * 1e6) / 1e6);
      }
    } else {
      for (let v = start; v >= end - 1e-9; v -= step) {
        values.push(Math.round(v * 1e6) / 1e6);
      }
    }
    try {
      previewResults = await previewCompuMethodCmd(editBuffer, values);
    } catch (e) {
      statusMessage.set(`❌ 预览失败: ${e}`);
    }
  }

  function addVerbPair() {
    editBuffer.verb_pairs = [...editBuffer.verb_pairs, { in_val: 0, verbal: '' }];
  }
  function removeVerbPair(idx: number) {
    editBuffer.verb_pairs = editBuffer.verb_pairs.filter((_, i) => i !== idx);
  }
  function addIntpPair() {
    editBuffer.intp_pairs = [...editBuffer.intp_pairs, { in_val: 0, out_val: 0 }];
  }
  function removeIntpPair(idx: number) {
    editBuffer.intp_pairs = editBuffer.intp_pairs.filter((_, i) => i !== idx);
  }

  function close() {
    showCompuMethodPanel.set(false);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  $effect(() => {
    if ($showCompuMethodPanel) {
      loadSummaries();
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $showCompuMethodPanel}
  <div class="overlay" onclick={close} role="dialog" aria-modal="true" transition:fade={{ duration: 100 }}>
    <div class="dialog" transition:fly={{ duration: 150, y: -20 }} onclick={(e) => e.stopPropagation()}>
      <div class="dialog-header">
        <h3>🔧 转换关系管理 (COMPU_METHOD)</h3>
        <button class="close-btn" onclick={close}>✖</button>
      </div>

      <div class="dialog-body">
        <div class="left-panel">
          <div class="left-toolbar">
            <input type="text" class="search-input" placeholder="搜索..." bind:value={searchQuery} />
            <button class="btn-sm btn-primary" onclick={startCreateNew}>➕ 新建</button>
            <button class="btn-sm" onclick={loadSummaries}>🔄</button>
          </div>

          <div class="cm-list">
            {#if isLoading && summaries.length === 0}
              <div class="empty-hint">加载中...</div>
            {:else if filteredSummaries.length === 0}
              <div class="empty-hint">无转换关系</div>
            {:else}
              {#each filteredSummaries as s (s.name)}
                <button
                  class="cm-item"
                  class:selected={selectedName === s.name && !isCreatingNew}
                  onclick={() => selectMethod(s.name)}
                >
                  <div class="cm-item-top">
                    <span class="cm-type-badge" style="background: {TYPE_COLORS[s.conversion_type] || '#666'}">
                      {TYPE_LABELS[s.conversion_type] || s.conversion_type}
                    </span>
                    <span class="cm-name">{s.name}</span>
                  </div>
                  <div class="cm-item-bottom">
                    <span class="cm-summary">{s.summary}</span>
                    {#if s.ref_count > 0}
                      <span class="cm-ref">引用 {s.ref_count}</span>
                    {/if}
                  </div>
                </button>
              {/each}
            {/if}
          </div>
        </div>

        <div class="right-panel">
          {#if isCreatingNew || detail}
            <div class="editor-form">
              <div class="form-row">
                <label class="form-label">
                  <span class="label-text">名称</span>
                  <input type="text" class="form-input" bind:value={editBuffer.name} disabled={isSaving} />
                </label>
                <label class="form-label">
                  <span class="label-text">类型</span>
                  <select class="form-select" bind:value={editBuffer.conversion_type} disabled={isSaving}>
                    <option value="LINEAR">LINEAR (线性)</option>
                    <option value="TAB_VERB">TAB_VERB (文字表)</option>
                    <option value="TAB_INTP">TAB_INTP (插值表)</option>
                    <option value="IDENTICAL">IDENTICAL (无转换)</option>
                  </select>
                </label>
              </div>

              <div class="form-row">
                <label class="form-label">
                  <span class="label-text">单位</span>
                  <input type="text" class="form-input" bind:value={editBuffer.unit} placeholder="如 °C, rpm, ms" disabled={isSaving} />
                </label>
                <label class="form-label">
                  <span class="label-text">描述</span>
                  <input type="text" class="form-input" bind:value={editBuffer.description} disabled={isSaving} />
                </label>
              </div>

              {#if editBuffer.conversion_type === 'LINEAR'}
                <div class="section-divider"><span class="section-label">线性参数</span></div>
                <div class="form-row">
                  <label class="form-label">
                    <span class="label-text">F (斜率)</span>
                    <input type="number" step="any" class="form-input" bind:value={editBuffer.f} disabled={isSaving} />
                  </label>
                  <label class="form-label">
                    <span class="label-text">OFFSET</span>
                    <input type="number" step="any" class="form-input" bind:value={editBuffer.offset} disabled={isSaving} />
                  </label>
                </div>
              {:else if editBuffer.conversion_type === 'IDENTICAL'}
                <div class="section-divider"><span class="section-label">无转换参数</span></div>
                <p class="hint-text">此类型不进行任何转换，物理值 = 原始值。</p>
              {:else if editBuffer.conversion_type === 'TAB_VERB'}
                <div class="section-divider"><span class="section-label">文字映射表</span></div>
                <label class="form-label full-width">
                  <span class="label-text">默认值 (未匹配时)</span>
                  <input type="text" class="form-input" bind:value={editBuffer.default_value} disabled={isSaving} />
                </label>
                <div class="pair-table">
                  <div class="pair-header">
                    <span>原始值</span>
                    <span>文字描述</span>
                    <span></span>
                  </div>
                  {#each editBuffer.verb_pairs as pair, idx}
                    <div class="pair-row">
                      <input type="number" step="1" class="pair-input" bind:value={pair.in_val} disabled={isSaving} />
                      <input type="text" class="pair-input" bind:value={pair.verbal} disabled={isSaving} />
                      <button class="btn-sm btn-danger" onclick={() => removeVerbPair(idx)} disabled={isSaving}>✖</button>
                    </div>
                  {/each}
                  <button class="btn-sm btn-add-row" onclick={addVerbPair} disabled={isSaving}>➕ 添加行</button>
                </div>
              {:else if editBuffer.conversion_type === 'TAB_INTP'}
                <div class="section-divider"><span class="section-label">插值表</span></div>
                <div class="pair-table">
                  <div class="pair-header">
                    <span>原始值 (X)</span>
                    <span>物理值 (Y)</span>
                    <span></span>
                  </div>
                  {#each editBuffer.intp_pairs as pair, idx}
                    <div class="pair-row">
                      <input type="number" step="any" class="pair-input" bind:value={pair.in_val} disabled={isSaving} />
                      <input type="number" step="any" class="pair-input" bind:value={pair.out_val} disabled={isSaving} />
                      <button class="btn-sm btn-danger" onclick={() => removeIntpPair(idx)} disabled={isSaving}>✖</button>
                    </div>
                  {/each}
                  <button class="btn-sm btn-add-row" onclick={addIntpPair} disabled={isSaving}>➕ 添加行</button>
                </div>
              {/if}

              <div class="section-divider"><span class="section-label">转换预览</span></div>
              <div class="preview-controls">
                <label class="form-label">
                  <span class="label-text">起始</span>
                  <input type="number" step="any" class="form-input preview-input" bind:value={previewRawStart} />
                </label>
                <label class="form-label">
                  <span class="label-text">结束</span>
                  <input type="number" step="any" class="form-input preview-input" bind:value={previewRawEnd} />
                </label>
                <label class="form-label">
                  <span class="label-text">步长</span>
                  <input type="number" step="any" class="form-input preview-input" bind:value={previewStep} />
                </label>
                <button class="btn-sm btn-primary" onclick={handlePreview}>🔍 预览</button>
              </div>
              {#if previewResults.length > 0}
                <div class="preview-table">
                  <div class="preview-header">
                    <span>原始值</span>
                    <span>{editBuffer.conversion_type === 'TAB_VERB' ? '文字' : '物理值'}</span>
                  </div>
                  <div class="preview-rows">
                    {#each previewResults as r}
                      <div class="preview-row">
                        <span class="preview-raw">{r.raw}</span>
                        <span class="preview-phys">
                          {#if r.verbal !== null}
                            {r.verbal}
                          {:else if r.physical !== null}
                            {r.physical.toFixed(4)}
                          {:else}
                            —
                          {/if}
                        </span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <div class="empty-editor">
              <span class="empty-icon">👈</span>
              <span>从左侧选择一个转换关系，或点击"新建"</span>
            </div>
          {/if}
        </div>
      </div>

      <div class="dialog-footer">
        {#if selectedName && !isCreatingNew}
          <button class="btn btn-danger-left" onclick={handleDelete} disabled={isSaving}>🗑️ 删除</button>
        {/if}
        <div class="footer-right">
          <button class="btn secondary" onclick={close}>关闭</button>
          {#if isCreatingNew || detail}
            <button class="btn primary" onclick={handleSave} disabled={!hasChanges || isSaving || !editBuffer.name.trim()}>
              {isSaving ? '保存中...' : '💾 保存'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 999;
  }

  .dialog {
    width: 90vw;
    height: 85vh;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 15px;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .dialog-body {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .left-panel {
    width: 300px;
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
  }

  .left-toolbar {
    display: flex;
    gap: 4px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }

  .search-input {
    flex: 1;
    padding: 4px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-size: 12px;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .cm-list {
    flex: 1;
    overflow-y: auto;
  }

  .empty-hint {
    padding: 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
  }

  .cm-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }

  .cm-item:hover {
    background: var(--bg-hover);
  }

  .cm-item.selected {
    background: var(--bg-active, rgba(100, 150, 255, 0.15));
    border-left: 3px solid var(--accent);
    padding-left: 9px;
  }

  .cm-item-top {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }

  .cm-type-badge {
    font-size: 9px;
    padding: 1px 6px;
    border-radius: 3px;
    color: white;
    white-space: nowrap;
  }

  .cm-name {
    font-size: 12px;
    font-family: monospace;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cm-item-bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-left: 2px;
  }

  .cm-summary {
    font-size: 10px;
    color: var(--text-muted);
  }

  .cm-ref {
    font-size: 10px;
    color: var(--accent);
  }

  .right-panel {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .editor-form {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .empty-editor {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .empty-icon {
    font-size: 32px;
  }

  .form-row {
    display: flex;
    gap: 12px;
    margin-bottom: 8px;
  }

  .form-label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .form-label.full-width {
    width: 100%;
    margin-bottom: 8px;
  }

  .label-text {
    font-size: 11px;
    color: var(--text-muted);
    min-width: 50px;
    white-space: nowrap;
  }

  .form-input, .form-select {
    flex: 1;
    padding: 4px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-size: 12px;
    font-family: monospace;
  }

  .form-input:focus, .form-select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .form-input:disabled, .form-select:disabled {
    opacity: 0.6;
  }

  .section-divider {
    display: flex;
    align-items: center;
    margin: 12px 0 8px 0;
    padding-top: 8px;
    border-top: 1px dashed var(--border);
  }

  .section-label {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .hint-text {
    font-size: 11px;
    color: var(--text-muted);
    padding: 4px 0;
  }

  .pair-table {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .pair-header {
    display: grid;
    grid-template-columns: 1fr 1fr 32px;
    gap: 4px;
    font-size: 10px;
    color: var(--text-muted);
    padding: 0 2px;
  }

  .pair-row {
    display: grid;
    grid-template-columns: 1fr 1fr 32px;
    gap: 4px;
    align-items: center;
  }

  .pair-input {
    padding: 4px 6px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text);
    font-size: 12px;
    font-family: monospace;
  }

  .pair-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .pair-input:disabled {
    opacity: 0.6;
  }

  .btn-sm {
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    white-space: nowrap;
  }

  .btn-sm:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn-sm:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-sm.btn-primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn-sm.btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-sm.btn-danger {
    background: transparent;
    border-color: transparent;
    color: #f44336;
    font-size: 12px;
    padding: 3px 6px;
  }

  .btn-sm.btn-danger:hover:not(:disabled) {
    background: rgba(244, 67, 54, 0.1);
  }

  .btn-add-row {
    align-self: flex-start;
    margin-top: 4px;
  }

  .preview-controls {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    margin-bottom: 8px;
  }

  .preview-input {
    max-width: 80px;
  }

  .preview-table {
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
  }

  .preview-header {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 6px 12px;
    background: var(--bg-hover);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .preview-rows {
    max-height: 200px;
    overflow-y: auto;
  }

  .preview-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 4px 12px;
    font-size: 12px;
    font-family: monospace;
    border-bottom: 1px solid var(--border);
  }

  .preview-row:last-child {
    border-bottom: none;
  }

  .preview-raw {
    color: var(--text-muted);
  }

  .preview-phys {
    color: var(--text);
    font-weight: 500;
  }

  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  .footer-right {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }

  .btn {
    padding: 6px 16px;
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

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .btn.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn.secondary {
    background: var(--bg);
  }

  .btn-danger-left {
    border-color: #f44336;
    color: #f44336;
  }

  .btn-danger-left:hover:not(:disabled) {
    background: rgba(244, 67, 54, 0.1);
  }
</style>
