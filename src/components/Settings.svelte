<script lang="ts">
  import { settingsOpen, storageFolder } from '$lib/stores';
  import { createEventDispatcher } from 'svelte';
  import { open, ask } from '@tauri-apps/plugin-dialog';

  const dispatch = createEventDispatcher<{
    folderChange: { folder: string };
    resetColors: void;
  }>();

  function close() {
    settingsOpen.set(false);
  }

  function backdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }

  async function changeFolder() {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === 'string') {
      dispatch('folderChange', { folder: result });
    }
  }

  async function resetColors() {
    const ok = await ask(
      'Rebuild dot order from file dates? This affects all devices once Dropbox syncs.',
      { title: 'Reset color order', kind: 'warning' }
    );
    if (ok) dispatch('resetColors');
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="overlay" onmousedown={backdropClick} role="presentation">
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <h2 id="settings-title">Settings</h2>

    <div class="field">
      <div class="label">Storage folder</div>
      <div class="row">
        <code class="path">{$storageFolder ?? ''}</code>
        <button type="button" onclick={changeFolder}>Change…</button>
      </div>
    </div>

    <div class="field">
      <button type="button" class="warn" onclick={resetColors}>Reset color order</button>
    </div>

    <div class="footer">
      <button type="button" class="primary" onclick={close}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--modal-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    width: 480px;
    max-width: calc(100% - 48px);
    background: var(--modal-bg);
    color: var(--fg);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    box-shadow: 0 10px 40px rgba(0,0,0,0.25);
  }
  h2 { margin: 0; font-size: 16px; font-weight: 600; }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .path {
    flex: 1;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 12px;
    padding: 6px 8px;
    border: 1px solid var(--bar-border);
    border-radius: 4px;
    background: var(--bar-bg);
    word-break: break-all;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
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
    font-family: inherit;
  }
  button:hover { opacity: 0.9; }
  button.primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  button.warn { color: #c0392b; }
</style>
