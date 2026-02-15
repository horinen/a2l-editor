<script lang="ts" generics="T">
  import { onMount, onDestroy } from 'svelte';
  
  interface Props {
    items: T[];
    itemHeight?: number;
    overscan?: number;
    children: import('svelte').Snippet<[T, number]>;
    onscroll?: (scrollTop: number) => void;
  }
  
  let {
    items,
    itemHeight = 32,
    overscan = 3,
    children,
    onscroll
  }: Props = $props();
  
  let container: HTMLDivElement;
  let scrollTop = $state(0);
  let containerHeight = $state(600);
  
  let startIndex = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
  let endIndex = $derived(Math.min(
    items.length,
    Math.ceil(scrollTop / itemHeight) + Math.ceil(containerHeight / itemHeight) + overscan + 1
  ));
  
  let visibleItems = $derived(items.slice(startIndex, endIndex));
  let totalHeight = $derived(items.length * itemHeight);
  let offsetY = $derived(startIndex * itemHeight);
  
  let resizeObserver: ResizeObserver | null = null;
  
  function handleScroll(e: Event) {
    scrollTop = (e.target as HTMLDivElement).scrollTop;
    onscroll?.(scrollTop);
  }
  
  onMount(() => {
    if (container) {
      containerHeight = container.clientHeight;
      resizeObserver = new ResizeObserver(entries => {
        containerHeight = entries[0].contentRect.height;
      });
      resizeObserver.observe(container);
    }
  });
  
  onDestroy(() => {
    resizeObserver?.disconnect();
  });
  
  export function scrollToIndex(index: number) {
    if (container) {
      const targetTop = index * itemHeight;
      container.scrollTo({
        top: targetTop,
        behavior: 'smooth'
      });
    }
  }
</script>

<div class="virtual-list" bind:this={container} onscroll={handleScroll}>
  <div class="content" style="height: {totalHeight}px;">
    <div class="viewport" style="transform: translateY({offsetY}px);">
      {#each visibleItems as item, i}
        {@render children(item, startIndex + i)}
      {/each}
    </div>
  </div>
</div>

<style>
  .virtual-list {
    height: 100%;
    overflow-y: auto;
    position: relative;
  }
  
  .content {
    position: relative;
    width: 100%;
  }
  
  .viewport {
    position: absolute;
    width: 100%;
    will-change: transform;
  }
</style>
