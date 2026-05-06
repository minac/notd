<script lang="ts">
  import { sortedNotes, activeFilename, theme, storageFolder } from '$lib/stores';
  import { colorForIndex } from '$lib/colors';
  import { readNote } from '$lib/fs';
  import { ask } from '@tauri-apps/plugin-dialog';
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher<{
    select: { filename: string };
    create: void;
    delete: { filename: string };
  }>();

  // Tooltip cache: filename → first 40 chars of body (or "Empty note").
  // Loaded lazily on first hover and cleared when meta changes.
  const tooltipCache = new Map<string, string>();
  $: $sortedNotes, tooltipCache.clear();

  let menuFor: string | null = null;
  let menuX = 0;
  let menuY = 0;

  function selectNote(filename: string) {
    closeMenu();
    if (filename !== $activeFilename) {
      dispatch('select', { filename });
    }
  }

  function newNote() {
    closeMenu();
    dispatch('create');
  }

  async function onContextMenu(e: MouseEvent, filename: string) {
    e.preventDefault();
    menuFor = filename;
    menuX = e.clientX;
    menuY = e.clientY;
  }

  function closeMenu() {
    menuFor = null;
  }

  async function confirmDelete(filename: string) {
    closeMenu();
    const ok = await ask('Delete this note? This cannot be undone.', {
      title: 'Delete note',
      kind: 'warning'
    });
    if (ok) dispatch('delete', { filename });
  }

  async function tooltipFor(filename: string): Promise<string> {
    const cached = tooltipCache.get(filename);
    if (cached !== undefined) return cached;
    const folder = $storageFolder;
    if (!folder) return '';
    try {
      const body = await readNote(folder, filename);
      const trimmed = body.replace(/\s+/g, ' ').trim();
      const value = trimmed.length === 0 ? 'Empty note' : trimmed.slice(0, 40);
      tooltipCache.set(filename, value);
      return value;
    } catch {
      return '';
    }
  }

  async function handleMouseEnter(e: MouseEvent, filename: string) {
    const el = e.currentTarget as HTMLElement;
    const value = await tooltipFor(filename);
    el.title = value;
  }
</script>

<svelte:window on:click={closeMenu} on:contextmenu={(e) => { if (menuFor) closeMenu(); }} />

<div class="dot-row" role="toolbar" aria-label="Notes">
  {#each $sortedNotes as note (note.filename)}
    {@const color = colorForIndex(note.createdIndex, $theme)}
    {@const isActive = note.filename === $activeFilename}
    <button
      class="dot"
      class:active={isActive}
      style="--c: {color};"
      type="button"
      aria-label="Open note"
      on:click={() => selectNote(note.filename)}
      on:contextmenu={(e) => onContextMenu(e, note.filename)}
      on:mouseenter={(e) => handleMouseEnter(e, note.filename)}
    ></button>
  {/each}
  <button class="add" type="button" aria-label="New note" on:click={newNote}>+</button>
</div>

{#if menuFor}
  <div class="menu" style="left: {menuX}px; top: {menuY}px;" role="menu">
    <button type="button" role="menuitem" on:click={() => menuFor && confirmDelete(menuFor)}>
      Delete note
    </button>
  </div>
{/if}

<style>
  .dot-row {
    height: 40px;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 12px;
    overflow-x: auto;
    overflow-y: hidden;
    flex-shrink: 0;
    background: var(--bar-bg);
    border-bottom: 1px solid var(--bar-border);
    scrollbar-width: none;
  }
  .dot-row::-webkit-scrollbar { display: none; }

  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: none;
    padding: 0;
    flex-shrink: 0;
    background: var(--c);
    cursor: pointer;
    box-shadow: 0 0 0 0 transparent;
    transition: box-shadow 60ms linear;
  }
  .dot.active {
    /* 2px gap then 2px ring */
    box-shadow: 0 0 0 2px var(--bar-bg), 0 0 0 4px var(--ring);
  }
  .dot:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--bar-bg), 0 0 0 4px var(--ring);
  }

  .add {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    background: transparent;
    border: none;
    padding: 0;
    color: var(--fg);
    opacity: 0.5;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 16px;
    line-height: 16px;
    cursor: pointer;
    text-align: center;
  }
  .add:hover { opacity: 1; }

  .menu {
    position: fixed;
    z-index: 200;
    background: var(--modal-bg);
    border: 1px solid var(--bar-border);
    border-radius: 4px;
    padding: 4px;
    min-width: 140px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.2);
  }
  .menu button {
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    padding: 6px 10px;
    font-size: 13px;
    color: var(--fg);
    cursor: pointer;
    border-radius: 3px;
    font-family: inherit;
  }
  .menu button:hover {
    background: var(--accent);
    color: white;
  }
</style>
