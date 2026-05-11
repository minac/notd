import { get } from 'svelte/store';
import {
  storageFolder,
  activeFilename,
  activeBody,
  lastSavedBody,
  lastKnownMtime,
  banner
} from './stores';
import { writeNote, getMtime } from './fs';

const SAVE_DEBOUNCE_MS = 500;

let saveTimer: ReturnType<typeof setTimeout> | null = null;
// Track which filename a queued save is for, so we don't save the wrong body.
let savingFilename: string | null = null;
let onDeleteRequest: ((filename: string) => Promise<void>) | null = null;

export function configureSaveManager(opts: {
  onDeleteRequest: (filename: string) => Promise<void>;
}) {
  onDeleteRequest = opts.onDeleteRequest;
}

export async function persistOrDelete(filename: string, value: string) {
  const folder = get(storageFolder);
  if (!folder) return;
  // Transition from non-empty on disk to empty body → remove the note.
  if (value === '' && get(lastSavedBody) !== '') {
    if (onDeleteRequest) await onDeleteRequest(filename);
    return;
  }
  try {
    await writeNote(filename, value);
    lastSavedBody.set(value);
    lastKnownMtime.set(await getMtime(filename).catch(() => Date.now()));
    const current = get(banner);
    if (current?.kind === 'error') banner.set(null);
  } catch (e) {
    banner.set({ kind: 'error', message: `Could not save: ${String(e)}`, filename });
  }
}

export async function flushPendingSave() {
  if (saveTimer === null || !savingFilename) return;
  clearTimeout(saveTimer);
  saveTimer = null;
  const filename = savingFilename;
  const value = get(activeBody);
  savingFilename = null;
  await persistOrDelete(filename, value);
}

export function scheduleSave() {
  const folder = get(storageFolder);
  const filename = get(activeFilename);
  if (!folder || !filename) return;
  if (saveTimer !== null) clearTimeout(saveTimer);
  savingFilename = filename;
  saveTimer = setTimeout(async () => {
    saveTimer = null;
    const value = get(activeBody);
    const targetFilename = savingFilename;
    savingFilename = null;
    if (!targetFilename) return;
    await persistOrDelete(targetFilename, value);
  }, SAVE_DEBOUNCE_MS);
}

export function disposeSaveManager() {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  savingFilename = null;
}
