<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    getDefaultStorageFolder,
    createDir,
    writeAppConfig,
    pathExists,
    setStorageFolder
  } from '$lib/fs';
  import { writeMeta } from '$lib/meta';

  const dispatch = createEventDispatcher<{ done: { folder: string } }>();

  let defaultFolder = $state('');
  let busy = $state(false);
  let error = $state('');

  onMount(async () => {
    defaultFolder = await getDefaultStorageFolder();
  });

  async function finalize(folder: string) {
    busy = true;
    error = '';
    try {
      const exists = await pathExists(folder);
      if (!exists) await createDir(folder);
      // Hand the canonical folder to Rust BEFORE the first storage call;
      // writeMeta below reads it from AppState rather than taking a
      // folder argument.
      await setStorageFolder(folder);
      await writeMeta({ version: 1, notes: [], nextIndex: 0 });
      await writeAppConfig(JSON.stringify({ storageFolder: folder }));
      dispatch('done', { folder });
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }

  async function chooseFolder() {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === 'string') await finalize(result);
  }

  function useDefault() {
    finalize(defaultFolder);
  }
</script>

<div class="wrap">
  <div class="card">
    <h1>Where should notd store your notes?</h1>
    <p class="sub">Notes are saved as plain .md files. Use a Dropbox folder if you want sync.</p>

    <div class="path">{defaultFolder || '…'}</div>

    <div class="row">
      <button type="button" class="primary" disabled={busy} onclick={useDefault}>Use default</button>
      <button type="button" disabled={busy} onclick={chooseFolder}>Choose folder…</button>
    </div>

    {#if error}
      <p class="err">{error}</p>
    {/if}
  </div>
</div>

<style>
  .wrap {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .card {
    width: 480px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h1 {
    font-size: 18px;
    font-weight: 600;
    margin: 0;
  }
  .sub {
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
  .path {
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 12px;
    padding: 8px 10px;
    border: 1px solid var(--bar-border);
    border-radius: 4px;
    background: var(--bar-bg);
    color: var(--fg);
    word-break: break-all;
  }
  .row {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }
  button {
    border: 1px solid var(--bar-border);
    background: var(--bar-bg);
    color: var(--fg);
    font-size: 13px;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover { opacity: 0.9; }
  button.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  button:disabled { opacity: 0.5; cursor: default; }
  .err {
    color: #c0392b;
    font-size: 12px;
    margin: 0;
  }
</style>
