<script lang="ts">
  import { banner } from '$lib/stores';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{
    retry: void;
    keepMine: void;
    useTheirs: void;
  }>();

  function dismiss() {
    banner.set(null);
  }
</script>

{#if $banner}
  <div class="banner" role="alert">
    <span class="msg">{$banner.message}</span>
    <span class="actions">
      {#if $banner.kind === 'error'}
        <button type="button" on:click={() => dispatch('retry')}>Retry</button>
      {:else if $banner.kind === 'conflict'}
        <button type="button" on:click={() => dispatch('keepMine')}>Keep mine</button>
        <button type="button" on:click={() => dispatch('useTheirs')}>Use theirs</button>
      {/if}
      <button type="button" class="close" aria-label="Dismiss" on:click={dismiss}>×</button>
    </span>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background: var(--banner-bg);
    color: var(--banner-fg);
    border-bottom: 1px solid var(--banner-border);
    font-size: 13px;
    flex-shrink: 0;
  }
  .msg {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  button {
    background: transparent;
    border: 1px solid currentColor;
    border-radius: 3px;
    padding: 2px 8px;
    color: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  button:hover { opacity: 0.8; }
  .close {
    border: none;
    font-size: 16px;
    line-height: 1;
    padding: 0 4px;
  }
</style>
