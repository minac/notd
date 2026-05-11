import { get } from 'svelte/store';
import {
  storageFolder,
  meta,
  activeFilename,
  activeBody,
  lastSavedBody,
  lastKnownMtime,
  banner,
  settingsOpen
} from './stores';
import {
  pathExists,
  readAppConfig,
  writeAppConfig,
  readNote,
  getMtime,
  createDir,
  setStorageFolder
} from './fs';
import { loadMeta } from './meta';
import { flushPendingSave } from './saveManager';

export type BootstrapResult =
  | { phase: 'setup' }
  | { phase: 'app' };

export async function persistConfig(folder: string, active: string | null) {
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

export async function loadActive(filename: string) {
  const folder = get(storageFolder);
  if (!folder) return;
  const body = await readNote(filename);
  const mtime = await getMtime(filename).catch(() => Date.now());
  activeFilename.set(filename);
  activeBody.set(body);
  lastSavedBody.set(body);
  lastKnownMtime.set(mtime);
  await persistConfig(folder, filename);
}

function clearActive() {
  activeFilename.set(null);
  activeBody.set('');
  lastSavedBody.set('');
  lastKnownMtime.set(0);
}

export async function bootstrap(): Promise<BootstrapResult> {
  try {
    const cfg = await loadConfig();
    const folder = cfg?.storageFolder;
    if (!folder) return { phase: 'setup' };

    const exists = await pathExists(folder);
    if (!exists) {
      try {
        await createDir(folder);
      } catch {
        return { phase: 'setup' };
      }
    }
    // Tell Rust the canonical folder before any storage call. From here
    // on, fs/meta wrappers don't take a folder — they read AppState.
    await setStorageFolder(folder);
    storageFolder.set(folder);

    const m = await loadMeta();
    meta.set(m);

    let initial: string | null = null;
    if (cfg?.activeFilename && m.notes.some((n) => n.filename === cfg.activeFilename)) {
      initial = cfg.activeFilename;
    } else if (m.notes.length > 0) {
      initial = [...m.notes].sort((a, b) => a.createdIndex - b.createdIndex)[0].filename;
    }

    if (initial) {
      await loadActive(initial);
    } else {
      clearActive();
    }

    return { phase: 'app' };
  } catch (e) {
    console.error('bootstrap failed:', e);
    return { phase: 'setup' };
  }
}

export async function handleSetupDone(folder: string): Promise<BootstrapResult> {
  try {
    // Setup.svelte already called setStorageFolder before writing the
    // initial meta. Re-call here defensively — it's idempotent and
    // makes the bootstrap and setup paths look the same.
    await setStorageFolder(folder);
    storageFolder.set(folder);
    const m = await loadMeta();
    meta.set(m);
    await persistConfig(folder, null);
    clearActive();
    return { phase: 'app' };
  } catch (e) {
    console.error('handleSetupDone failed:', e);
    return { phase: 'setup' };
  }
}

export async function handleSettingsFolderChange(folder: string) {
  try {
    await flushPendingSave();
    const exists = await pathExists(folder);
    if (!exists) await createDir(folder);
    // Swap Rust's canonical folder before any storage call. Until this
    // returns, loadMeta/loadActive would still operate on the previous
    // folder.
    await setStorageFolder(folder);
    storageFolder.set(folder);
    const m = await loadMeta();
    meta.set(m);
    const sorted = [...m.notes].sort((a, b) => a.createdIndex - b.createdIndex);
    if (sorted.length > 0) {
      await loadActive(sorted[0].filename);
    } else {
      clearActive();
      await persistConfig(folder, null);
    }
    settingsOpen.set(false);
  } catch (e) {
    console.error('handleSettingsFolderChange failed:', e);
    banner.set({ kind: 'error', message: `Could not switch folder: ${String(e)}` });
  }
}
