import { writable, derived, type Writable, type Readable } from 'svelte/store';
import type { Meta, MetaEntry } from './meta';
import { EMPTY_META } from './meta';

export type Theme = 'light' | 'dark';
export type Mode = 'edit' | 'preview';

export interface BannerState {
  kind: 'error' | 'conflict';
  message: string;
  filename?: string;
}

export const storageFolder: Writable<string | null> = writable(null);
export const meta: Writable<Meta> = writable(EMPTY_META);

export const activeFilename: Writable<string | null> = writable(null);

export const theme: Writable<Theme> = writable('light');
export const mode: Writable<Mode> = writable('edit');
export const banner: Writable<BannerState | null> = writable(null);

// Editor body bound to <textarea>; lastSavedBody tracks what's persisted on disk
// so we can detect dirty state and conflicts.
export const activeBody: Writable<string> = writable('');
export const lastSavedBody: Writable<string> = writable('');
export const lastKnownMtime: Writable<number> = writable(0);

// Saved scroll position from edit mode, restored when toggling back from preview.
export const editorScrollTop: Writable<number> = writable(0);

// Settings modal visibility
export const settingsOpen: Writable<boolean> = writable(false);

export const dirty: Readable<boolean> = derived(
  [activeBody, lastSavedBody],
  ([$body, $saved]) => $body !== $saved
);

export const sortedNotes: Readable<MetaEntry[]> = derived(meta, ($meta) =>
  [...$meta.notes].sort((a, b) => a.createdIndex - b.createdIndex)
);
