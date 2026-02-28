<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { readFile } from '@tauri-apps/plugin-fs';
  import * as XLSX from 'xlsx';
  import { elfEntries, a2lVariables } from '$lib/stores';
  import { saveA2lChanges } from '$lib/commands';
  import type { A2lVariableEdit, ExcelImportRow, A2lEntry, ExcelImportResult } from '$lib/types';

  interface Props {
    visible: boolean;
    onclose: () => void;
    onresult?: (result: ExcelImportResult) => void;
  }

  let { visible, onclose, onresult }: Props = $props();

  let importing = $state(false);
  let error = $state<string | null>(null);

  async function selectAndImport() {
    error = null;
    importing = true;

    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });

      if (!selected) {
        importing = false;
        return;
      }

      const filePath = typeof selected === 'string' ? selected : (selected as { path: string }).path;
      if (!filePath) {
        importing = false;
        return;
      }

      const data = await readFile(filePath);
      const workbook = XLSX.read(data, { type: 'array' });
      const sheetName = workbook.SheetNames[0];
      const worksheet = workbook.Sheets[sheetName];
      const rows = XLSX.utils.sheet_to_json(worksheet) as ExcelImportRow[];

      if (rows.length === 0) {
        error = 'Excel 文件为空';
        importing = false;
        return;
      }

      const firstRow = rows[0];
      if (!('名称' in firstRow) || !('link' in firstRow) || !('变量类型' in firstRow)) {
        error = 'Excel 格式错误：缺少必要的列（名称、link、变量类型）';
        importing = false;
        return;
      }

      const elfMap = new Map<string, A2lEntry>();
      for (const entry of $elfEntries) {
        elfMap.set(entry.full_name, entry);
      }

      const existingA2lNames = new Set($a2lVariables.map(v => v.name));

      const edits: A2lVariableEdit[] = [];
      const notFound: string[] = [];
      let skipped = 0;

      for (const row of rows) {
        const name = row['名称']?.trim();
        const link = row['link']?.trim();
        const varType = row['变量类型']?.trim();

        if (!name || !link || !varType) {
          skipped++;
          continue;
        }

        let exportMode: 'measurement' | 'characteristic';
        if (varType === '观测') {
          exportMode = 'measurement';
        } else if (varType === '标定') {
          exportMode = 'characteristic';
        } else {
          skipped++;
          continue;
        }

        const elfEntry = elfMap.get(link);
        if (!elfEntry) {
          notFound.push(link);
          continue;
        }

        if (existingA2lNames.has(name)) {
          edits.push({
            action: 'delete',
            originalName: name,
          });
        }

        edits.push({
          action: 'add',
          originalName: name,
          symbol_link: link,
          exportMode,
          entry: {
            index: elfEntry.index,
            full_name: name,
            address: elfEntry.address,
            size: elfEntry.size,
            a2l_type: elfEntry.a2l_type,
            type_name: elfEntry.type_name,
            bit_offset: elfEntry.bit_offset,
            bit_size: elfEntry.bit_size,
            symbol_link: link,
          },
        });
      }

      if (edits.length === 0) {
        error = notFound.length > 0 
          ? `没有可导入的变量，${notFound.length} 个符号在 ELF 中未找到` 
          : '没有可导入的变量';
        importing = false;
        return;
      }

      const result = await saveA2lChanges(edits);
      
      const importResult: ExcelImportResult = {
        imported: result.added,
        skipped: result.skipped + skipped,
        notFound,
      };

      onresult?.(importResult);
      onclose();
    } catch (e) {
      error = `导入失败: ${e}`;
    } finally {
      importing = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    }
  }
</script>

{#if visible}
  <div class="overlay" onclick={onclose} onkeydown={handleKeydown}>
    <div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog">
      <div class="header">
        <span class="title">导入 Excel</span>
        <button class="close-btn" onclick={onclose}>✕</button>
      </div>

      <div class="content">
        <p class="hint">
          从 Excel 文件批量导入 A2L 变量。<br>
          Excel 格式：名称 | link | 变量类型 | 转换关系
        </p>

        {#if error}
          <p class="error">{error}</p>
        {/if}
      </div>

      <div class="footer">
        <button class="btn secondary" onclick={onclose} disabled={importing}>取消</button>
        <button class="btn primary" onclick={selectAndImport} disabled={importing}>
          {importing ? '导入中...' : '选择文件并导入'}
        </button>
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
    z-index: 1000;
  }

  .dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    min-width: 400px;
    max-width: 500px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    font-weight: 500;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
  }

  .close-btn:hover {
    color: var(--text);
  }

  .content {
    padding: 16px;
  }

  .hint {
    font-size: 13px;
    color: var(--text-muted);
    margin-bottom: 12px;
    line-height: 1.5;
  }

  .error {
    color: #ef4444;
    font-size: 13px;
    margin: 0;
    padding: 8px 12px;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 4px;
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--accent);
    border: none;
    color: white;
  }

  .btn.secondary {
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--text);
  }

  .btn:not(:disabled):hover {
    opacity: 0.85;
  }
</style>
