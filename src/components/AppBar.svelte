<script lang="ts">
  import { mode, activeBody } from '$lib/stores';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{ clear: void }>();

  $: hasText = $activeBody.length > 0;
  $: toggleLabel = $mode === 'edit' ? 'Preview' : 'Edit';

  function toggle() {
    mode.update((m) => (m === 'edit' ? 'preview' : 'edit'));
  }

  function clear() {
    dispatch('clear');
  }

  async function copy() {
    if (!hasText) return;
    try {
      await navigator.clipboard.writeText($activeBody);
    } catch {
      // best-effort; ignore
    }
  }
</script>

<div class="bar">
  <button class="btn" type="button" disabled={!hasText} on:click={copy}>
    Copy to clipboard
  </button>
  <button class="btn" type="button" disabled={!hasText} on:click={clear}>
    Clear text
  </button>
  <button class="btn" type="button" on:click={toggle}>{toggleLabel}</button>
</div>

<style>
  .bar {
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    padding-right: 12px;
    border-bottom: 1px solid var(--bar-border);
    background: var(--bar-bg);
    flex-shrink: 0;
  }
  .btn {
    background: none;
    border: none;
    padding: 4px 6px;
    color: var(--accent);
    font-size: 13px;
    cursor: pointer;
  }
  .btn:hover { opacity: 0.85; }
  .btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
</style>
