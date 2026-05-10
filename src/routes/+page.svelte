<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import {
    storageFolder,
    meta,
    activeFilename,
    mode,
    banner,
    activeBody,
    lastSavedBody,
    lastKnownMtime,
    settingsOpen,
    sortedNotes
  } from '$lib/stores';
  import {
    pathExists,
    readAppConfig,
    writeAppConfig,
    readNote,
    writeNote,
    deleteNote,
    listMarkdownFiles,
    getMtime,
    createDir
  } from '$lib/fs';
  import {
    loadMeta,
    addNoteToMeta,
    removeNoteFromMeta,
    writeMeta,
    deleteMetaFile,
    rebuildMeta
  } from '$lib/meta';
  import { generateFilename, resolveCollision } from '$lib/filename';
  import { attachShortcuts } from '$lib/shortcuts';

  import DotRow from '../components/DotRow.svelte';
  import AppBar from '../components/AppBar.svelte';
  import Editor from '../components/Editor.svelte';
  import Preview from '../components/Preview.svelte';
  import Banner from '../components/Banner.svelte';
  import Setup from '../components/Setup.svelte';
  import Settings from '../components/Settings.svelte';
  import EmptyState from '../components/EmptyState.svelte';

  type Phase = 'loading' | 'setup' | 'app';

  let phase: Phase = 'loading';
  let editorRef: Editor | undefined;

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  const SAVE_DEBOUNCE_MS = 500;

  // Track which filename a queued save is for, so we don't save the wrong body.
  let savingFilename: string | null = null;

  let detachShortcuts: (() => void) | undefined;
  let unlistenFocus: (() => void) | undefined;

  async function persistConfig(folder: string, active: string | null) {
    await writeAppConfig(JSON.stringify({ storageFolder: folder, activeFilename: active }));
  }

  async function loadConfig(): Promise<{ storageFolder?: string; activeFilename?: string } | null> {
    const raw = await readAppConfig();
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw);
      if (typeof parsed === 'object' && parsed !== null) return parsed;
    } catch {}
    return null;
  }

  async function bootstrap() {
    try {
      const cfg = await loadConfig();
      const folder = cfg?.storageFolder;
      if (!folder) {
        phase = 'setup';
        return;
      }
      const exists = await pathExists(folder);
      if (!exists) {
        try {
          await createDir(folder);
        } catch {
          phase = 'setup';
          return;
        }
      }
      storageFolder.set(folder);

      const m = await loadMeta(folder);
      meta.set(m);

      let initial: string | null = null;
      if (cfg?.activeFilename && m.notes.some((n) => n.filename === cfg.activeFilename)) {
        initial = cfg.activeFilename;
      } else if (m.notes.length > 0) {
        initial = [...m.notes].sort((a, b) => a.createdIndex - b.createdIndex)[0].filename;
      }

      if (initial) {
        await loadActive(folder, initial);
      } else {
        activeFilename.set(null);
        activeBody.set('');
        lastSavedBody.set('');
        lastKnownMtime.set(0);
      }

      phase = 'app';
      await tick();
      editorRef?.focus();
    } catch (e) {
      console.error('bootstrap failed:', e);
      phase = 'setup';
    }
  }

  async function loadActive(folder: string, filename: string) {
    const body = await readNote(folder, filename);
    const mtime = await getMtime(folder, filename).catch(() => Date.now());
    activeFilename.set(filename);
    activeBody.set(body);
    lastSavedBody.set(body);
    lastKnownMtime.set(mtime);
    await persistConfig(folder, filename);
  }

  async function persistOrDelete(filename: string, value: string) {
    const folder = $storageFolder;
    if (!folder) return;
    // Transition from non-empty on disk to empty body → remove the note.
    if (value === '' && $lastSavedBody !== '') {
      await handleDelete(filename);
      return;
    }
    try {
      await writeNote(folder, filename, value);
      lastSavedBody.set(value);
      lastKnownMtime.set(await getMtime(folder, filename).catch(() => Date.now()));
      if ($banner?.kind === 'error') banner.set(null);
    } catch (e) {
      banner.set({ kind: 'error', message: `Could not save: ${String(e)}`, filename });
    }
  }

  async function flushPendingSave() {
    if (saveTimer === null || !savingFilename) return;
    clearTimeout(saveTimer);
    saveTimer = null;
    const filename = savingFilename;
    const value = $activeBody;
    savingFilename = null;
    await persistOrDelete(filename, value);
  }

  function scheduleSave() {
    const folder = $storageFolder;
    const filename = $activeFilename;
    if (!folder || !filename) return;
    if (saveTimer !== null) clearTimeout(saveTimer);
    savingFilename = filename;
    saveTimer = setTimeout(async () => {
      saveTimer = null;
      const value = $activeBody;
      const targetFilename = savingFilename;
      savingFilename = null;
      if (!targetFilename) return;
      await persistOrDelete(targetFilename, value);
    }, SAVE_DEBOUNCE_MS);
  }

  function handleClear() {
    if (!$activeFilename) return;
    activeBody.set('');
    scheduleSave();
  }

  async function selectNote(filename: string) {
    if (filename === $activeFilename) return;
    await flushPendingSave();
    const folder = $storageFolder;
    if (!folder) return;
    await loadActive(folder, filename);
    banner.set(null);
    mode.set('edit');
    await tick();
    editorRef?.focus();
  }

  async function createNote() {
    const folder = $storageFolder;
    if (!folder) return;
    await flushPendingSave();
    const existing = new Set($meta.notes.map((n) => n.filename));
    const filename = resolveCollision(generateFilename(), existing);
    try {
      await writeNote(folder, filename, '');
      const { meta: nextMeta } = addNoteToMeta($meta, filename);
      meta.set(nextMeta);
      await writeMeta(folder, nextMeta);
      await loadActive(folder, filename);
      banner.set(null);
      mode.set('edit');
      await tick();
      editorRef?.focus();
    } catch (e) {
      banner.set({ kind: 'error', message: `Could not create note: ${String(e)}` });
    }
  }

  async function handleDelete(filename: string) {
    const folder = $storageFolder;
    if (!folder) return;

    // Capture the deleted note's createdIndex before removal so we can pick
    // the nearest neighbor (prefer previous, then next) accurately.
    const deletedEntry = $meta.notes.find((n) => n.filename === filename);
    const deletedIndex = deletedEntry?.createdIndex ?? null;

    try {
      await deleteNote(folder, filename);
      const nextMeta = removeNoteFromMeta($meta, filename);
      meta.set(nextMeta);
      await writeMeta(folder, nextMeta);

      if ($activeFilename === filename) {
        const target = pickNeighbor(nextMeta.notes, deletedIndex);
        if (target) {
          await loadActive(folder, target);
        } else {
          activeFilename.set(null);
          activeBody.set('');
          lastSavedBody.set('');
          lastKnownMtime.set(0);
          await persistConfig(folder, null);
        }
      }
    } catch (e) {
      banner.set({ kind: 'error', message: `Could not delete: ${String(e)}`, filename });
    }
  }

  function pickNeighbor(
    notes: { filename: string; createdIndex: number }[],
    deletedIndex: number | null
  ): string | null {
    if (notes.length === 0) return null;
    if (deletedIndex === null) {
      const sorted = [...notes].sort((a, b) => a.createdIndex - b.createdIndex);
      return sorted[0].filename;
    }
    let prev: { filename: string; createdIndex: number } | null = null;
    let next: { filename: string; createdIndex: number } | null = null;
    for (const n of notes) {
      if (n.createdIndex < deletedIndex) {
        if (!prev || n.createdIndex > prev.createdIndex) prev = n;
      } else if (n.createdIndex > deletedIndex) {
        if (!next || n.createdIndex < next.createdIndex) next = n;
      }
    }
    return (prev ?? next)?.filename ?? null;
  }

  async function refreshFromDisk() {
    const folder = $storageFolder;
    if (!folder) return;
    try {
      const m = await loadMeta(folder);
      meta.set(m);

      const active = $activeFilename;
      if (active) {
        const stillExists = m.notes.some((n) => n.filename === active);
        if (!stillExists) {
          const sorted = [...m.notes].sort((a, b) => a.createdIndex - b.createdIndex);
          if (sorted.length > 0) await loadActive(folder, sorted[0].filename);
          else {
            activeFilename.set(null);
            activeBody.set('');
            lastSavedBody.set('');
            lastKnownMtime.set(0);
          }
          return;
        }
        const mtime = await getMtime(folder, active).catch(() => 0);
        if (mtime > $lastKnownMtime) {
          const isDirty = $activeBody !== $lastSavedBody;
          if (isDirty) {
            banner.set({
              kind: 'conflict',
              message: 'This note was changed elsewhere.',
              filename: active
            });
          } else {
            const body = await readNote(folder, active);
            activeBody.set(body);
            lastSavedBody.set(body);
            lastKnownMtime.set(mtime);
          }
        }
      }
    } catch {
      // ignore — folder may have been temporarily unmounted
    }
  }

  async function bannerRetry() {
    if ($banner?.kind !== 'error') return;
    banner.set(null);
    scheduleSave();
  }

  async function bannerKeepMine() {
    if ($banner?.kind !== 'conflict') return;
    const folder = $storageFolder;
    const filename = $banner.filename ?? $activeFilename;
    banner.set(null);
    if (!folder || !filename) return;
    try {
      await writeNote(folder, filename, $activeBody);
      lastSavedBody.set($activeBody);
      lastKnownMtime.set(await getMtime(folder, filename).catch(() => Date.now()));
    } catch (e) {
      banner.set({ kind: 'error', message: `Could not save: ${String(e)}`, filename });
    }
  }

  async function bannerUseTheirs() {
    if ($banner?.kind !== 'conflict') return;
    const folder = $storageFolder;
    const filename = $banner.filename ?? $activeFilename;
    banner.set(null);
    if (!folder || !filename) return;
    try {
      const body = await readNote(folder, filename);
      const mtime = await getMtime(folder, filename).catch(() => Date.now());
      activeBody.set(body);
      lastSavedBody.set(body);
      lastKnownMtime.set(mtime);
    } catch (e) {
      banner.set({ kind: 'error', message: `Could not load: ${String(e)}`, filename });
    }
  }

  async function handleSetupDone(detail: { folder: string }) {
    try {
      storageFolder.set(detail.folder);
      const m = await loadMeta(detail.folder);
      meta.set(m);
      await persistConfig(detail.folder, null);
      activeFilename.set(null);
      activeBody.set('');
      lastSavedBody.set('');
      lastKnownMtime.set(0);
      phase = 'app';
    } catch (e) {
      console.error('handleSetupDone failed:', e);
      phase = 'setup';
    }
  }

  async function handleSettingsFolderChange(detail: { folder: string }) {
    const folder = detail.folder;
    try {
      await flushPendingSave();
      const exists = await pathExists(folder);
      if (!exists) await createDir(folder);
      storageFolder.set(folder);
      const m = await loadMeta(folder);
      meta.set(m);
      const sorted = [...m.notes].sort((a, b) => a.createdIndex - b.createdIndex);
      if (sorted.length > 0) {
        await loadActive(folder, sorted[0].filename);
      } else {
        activeFilename.set(null);
        activeBody.set('');
        lastSavedBody.set('');
        lastKnownMtime.set(0);
        await persistConfig(folder, null);
      }
      settingsOpen.set(false);
    } catch (e) {
      console.error('handleSettingsFolderChange failed:', e);
      banner.set({ kind: 'error', message: `Could not switch folder: ${String(e)}` });
    }
  }

  async function handleResetColors() {
    const folder = $storageFolder;
    if (!folder) return;
    await deleteMetaFile(folder);
    const m = await rebuildMeta(folder);
    meta.set(m);
    settingsOpen.set(false);
  }

  function openSettings() {
    settingsOpen.set(true);
  }

  onMount(() => {
    bootstrap();

    detachShortcuts = attachShortcuts({
      newNote: createNote,
      togglePreview: () => mode.update((m) => (m === 'edit' ? 'preview' : 'edit')),
      openSettings
    });

    (async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const win = getCurrentWindow();
        unlistenFocus = await win.onFocusChanged(({ payload }) => {
          if (payload === true) {
            refreshFromDisk();
            // Always re-focus editor on window focus
            tick().then(() => editorRef?.focus());
          }
        });
      } catch {
        const handler = () => {
          refreshFromDisk();
          tick().then(() => editorRef?.focus());
        };
        window.addEventListener('focus', handler);
        unlistenFocus = () => window.removeEventListener('focus', handler);
      }
    })();
  });

  onDestroy(() => {
    if (detachShortcuts) detachShortcuts();
    if (unlistenFocus) unlistenFocus();
    if (saveTimer !== null) clearTimeout(saveTimer);
  });

  // When mode flips back to edit, focus the textarea on next tick.
  $: if (phase === 'app' && $mode === 'edit') {
    tick().then(() => editorRef?.focus());
  }
</script>

{#if phase === 'loading'}
  <div class="loading"></div>
{:else if phase === 'setup'}
  <Setup on:done={(e) => handleSetupDone(e.detail)} />
{:else}
  <div class="shell">
    <DotRow
      on:select={(e) => selectNote(e.detail.filename)}
      on:create={createNote}
      on:delete={(e) => handleDelete(e.detail.filename)}
    />

    {#if $activeFilename}
      <AppBar on:clear={handleClear} />
      <Banner
        on:retry={bannerRetry}
        on:keepMine={bannerKeepMine}
        on:useTheirs={bannerUseTheirs}
      />
      {#if $mode === 'edit'}
        <Editor bind:this={editorRef} onInput={scheduleSave} />
      {:else}
        <Preview />
      {/if}
    {:else}
      <Banner
        on:retry={bannerRetry}
        on:keepMine={bannerKeepMine}
        on:useTheirs={bannerUseTheirs}
      />
      <EmptyState />
    {/if}
  </div>

  {#if $settingsOpen}
    <Settings
      on:folderChange={(e) => handleSettingsFolderChange(e.detail)}
      on:resetColors={handleResetColors}
    />
  {/if}
{/if}

<style>
  .shell {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .loading {
    height: 100vh;
    background: var(--bg);
  }
</style>
