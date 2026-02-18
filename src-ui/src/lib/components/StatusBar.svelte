<script lang="ts">
  import { derived } from 'svelte/store';
  import { 
    elfPath, elfSelectedIndices, a2lPath, 
    a2lSelectedIndices, statusMessage
  } from '$lib/stores';

  const hint = derived(
    [elfPath, elfSelectedIndices, a2lPath, a2lSelectedIndices, statusMessage],
    ([$elfPath, $elfSelected, $a2lPath, $a2lSelected, $status]) => {
      if ($status && !$status.startsWith('💡')) return $status;
      
      if (!$elfPath) return '💡 文件 → 打开 ELF 开始使用';
      if ($a2lSelected.size > 0) return '💡 右键 → 删除所选变量';
      if ($elfSelected.size > 0 && !$a2lPath) return '⚠️ 请先选择目标 A2L 文件';
      if ($elfSelected.size > 0) return '💡 右键 → 添加为观测/标定变量';
      return '💡 单击选择变量，右键打开菜单';
    }
  );
</script>

<div class="status-bar">{$hint}</div>

<style>
  .status-bar {
    padding: 8px 16px;
    background: var(--bg);
    border-top: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-muted);
  }
</style>
