import { invoke } from '@tauri-apps/api/core';
import { listMarkdownFiles } from './fs';

export interface MetaEntry {
  filename: string;
  createdIndex: number;
}

export interface Meta {
  version: number;
  notes: MetaEntry[];
  nextIndex: number;
}

export const CURRENT_META_VERSION = 1;

export const EMPTY_META: Meta = { version: CURRENT_META_VERSION, notes: [], nextIndex: 0 };

export async function readMetaRaw(): Promise<string | null> {
  return invoke<string | null>('read_meta');
}

// Returns the contents of `.notd-meta.json.bak` if Rust finds one. Used
// only as a fallback when the primary meta is missing or invalid.
export async function readMetaBakRaw(): Promise<string | null> {
  return invoke<string | null>('read_meta_bak');
}

export async function writeMeta(meta: Meta): Promise<void> {
  await invoke('write_meta', { json: JSON.stringify(meta, null, 2) });
}

export async function deleteMetaFile(): Promise<void> {
  await invoke('delete_meta');
}

export async function rebuildMeta(): Promise<Meta> {
  const files = await listMarkdownFiles();
  files.sort((a, b) => a.mtime_ms - b.mtime_ms);
  const meta: Meta = {
    version: CURRENT_META_VERSION,
    notes: files.map((f, i) => ({ filename: f.filename, createdIndex: i })),
    nextIndex: files.length
  };
  await writeMeta(meta);
  return meta;
}

// Called from loadMeta() between JSON.parse and isValidMeta. Receives anything
// (parsed JSON of unknown shape) and returns a value that should then be passed
// to isValidMeta. Must be a no-op for already-current-version metas.
function migrate(raw: unknown): unknown {
  if (!raw || typeof raw !== 'object') return raw;
  const m = raw as Record<string, unknown>;
  if (typeof m.version !== 'number') return raw;
  let current = m;
  // Future: when we bump schema, add transforms here.
  // Example shape, leave commented as guidance:
  // if (current.version === 1) {
  //   current = { ...current, version: 2, /* new field */ };
  // }
  return current;
}

function isValidMeta(value: unknown): value is Meta {
  if (typeof value !== 'object' || value === null) return false;
  const m = value as Partial<Meta>;
  if (typeof m.version !== 'number') return false;
  if (!Array.isArray(m.notes)) return false;
  if (typeof m.nextIndex !== 'number') return false;
  if (!Number.isInteger(m.nextIndex) || m.nextIndex < 0) return false;
  const seenIndices = new Set<number>();
  const seenFilenames = new Set<string>();
  let maxIndex = -1;
  for (const n of m.notes) {
    if (typeof n?.filename !== 'string' || n.filename === '') return false;
    if (typeof n?.createdIndex !== 'number') return false;
    if (!Number.isInteger(n.createdIndex) || n.createdIndex < 0) return false;
    if (seenIndices.has(n.createdIndex)) return false;
    if (seenFilenames.has(n.filename)) return false;
    seenIndices.add(n.createdIndex);
    seenFilenames.add(n.filename);
    if (n.createdIndex > maxIndex) maxIndex = n.createdIndex;
  }
  if (m.nextIndex <= maxIndex) return false;
  return true;
}

// Try to parse and validate a raw meta JSON blob. Returns the Meta on
// success, null on any failure (parse error, schema mismatch, invariant
// violation). Used by both the primary-meta and bak fallback paths.
function tryParseMeta(raw: string | null): Meta | null {
  if (raw === null) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  const migrated = migrate(parsed);
  if (!isValidMeta(migrated)) return null;
  // Forward-compat guard: a meta written by a newer build of the app could pass
  // structural validation but carry a version we don't understand. Treat as
  // invalid so loadMeta falls through to bak/rebuild rather than silently
  // writing data the newer app expects.
  if (migrated.version !== CURRENT_META_VERSION) return null;
  return migrated;
}

export async function loadMeta(): Promise<Meta> {
  const primary = tryParseMeta(await readMetaRaw());
  if (primary) return reconcile(primary);

  // Primary missing or corrupt. Try the bak snapshot before falling
  // through to a full rebuild — the bak preserves the monotonic
  // `createdIndex` invariant, while rebuild restarts from 0 and shuffles
  // every dot's color.
  const bak = tryParseMeta(await readMetaBakRaw());
  if (bak) return reconcile(bak);

  return rebuildMeta();
}

async function reconcile(meta: Meta): Promise<Meta> {
  const files = await listMarkdownFiles();
  const onDisk = new Set(files.map((f) => f.filename));
  const inMeta = new Set(meta.notes.map((n) => n.filename));

  const surviving = meta.notes.filter((n) => onDisk.has(n.filename));

  let nextIndex = meta.nextIndex;
  for (const n of surviving) {
    if (n.createdIndex >= nextIndex) nextIndex = n.createdIndex + 1;
  }

  const newFiles = files
    .filter((f) => !inMeta.has(f.filename))
    .sort((a, b) => a.mtime_ms - b.mtime_ms);

  const notes: MetaEntry[] = [...surviving];
  for (const f of newFiles) {
    notes.push({ filename: f.filename, createdIndex: nextIndex });
    nextIndex++;
  }

  const reconciled: Meta = { version: CURRENT_META_VERSION, notes, nextIndex };
  const changed =
    notes.length !== meta.notes.length ||
    nextIndex !== meta.nextIndex ||
    notes.some((n, i) => {
      const old = meta.notes[i];
      return !old || old.filename !== n.filename || old.createdIndex !== n.createdIndex;
    });

  if (changed) {
    await writeMeta(reconciled);
  }
  return reconciled;
}

export function addNoteToMeta(
  meta: Meta,
  filename: string
): { meta: Meta; createdIndex: number } {
  const createdIndex = meta.nextIndex;
  return {
    meta: {
      version: CURRENT_META_VERSION,
      notes: [...meta.notes, { filename, createdIndex }],
      nextIndex: meta.nextIndex + 1
    },
    createdIndex
  };
}

export function removeNoteFromMeta(meta: Meta, filename: string): Meta {
  return {
    version: CURRENT_META_VERSION,
    notes: meta.notes.filter((n) => n.filename !== filename),
    nextIndex: meta.nextIndex
  };
}
