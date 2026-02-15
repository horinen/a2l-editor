<script lang="ts">
  import { isLoading } from '$lib/stores';
  import { fade } from 'svelte/transition';
</script>

{#if $isLoading}
  <div class="overlay" transition:fade={{ duration: 200 }}>
    <div class="spinner-container">
      <div class="spinner"></div>
      <span class="text">加载中...</span>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    pointer-events: none;
  }
  
  .spinner-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
  }
  
  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  
  .text {
    color: var(--text);
    font-size: 14px;
  }
  
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
