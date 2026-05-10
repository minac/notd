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

export const EMPTY_META: Meta = { version: 1, notes: [], nextIndex: 0 };

export async function readMetaRaw(folder: string): Promise<string | null> {
  return invoke<string | null>('read_meta', { folder });
}

export async function writeMeta(folder: string, meta: Meta): Promise<void> {
  await invoke('write_meta', { folder, json: JSON.stringify(meta, null, 2) });
}

export async function deleteMetaFile(folder: string): Promise<void> {
  await invoke('delete_meta', { folder });
}

export async function rebuildMeta(folder: string): Promise<Meta> {
  const files = await listMarkdownFiles(folder);
  files.sort((a, b) => a.mtime_ms - b.mtime_ms);
  const meta: Meta = {
    version: 1,
    notes: files.map((f, i) => ({ filename: f.filename, createdIndex: i })),
    nextIndex: files.length
  };
  await writeMeta(folder, meta);
  return meta;
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

export async function loadMeta(folder: string): Promise<Meta> {
  const raw = await readMetaRaw(folder);
  if (raw === null) {
    return rebuildMeta(folder);
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return rebuildMeta(folder);
  }
  if (!isValidMeta(parsed)) {
    return rebuildMeta(folder);
  }
  return reconcile(folder, parsed);
}

async function reconcile(folder: string, meta: Meta): Promise<Meta> {
  const files = await listMarkdownFiles(folder);
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

  const reconciled: Meta = { version: 1, notes, nextIndex };
  const changed =
    notes.length !== meta.notes.length ||
    nextIndex !== meta.nextIndex ||
    notes.some((n, i) => {
      const old = meta.notes[i];
      return !old || old.filename !== n.filename || old.createdIndex !== n.createdIndex;
    });

  if (changed) {
    await writeMeta(folder, reconciled);
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
      version: 1,
      notes: [...meta.notes, { filename, createdIndex }],
      nextIndex: meta.nextIndex + 1
    },
    createdIndex
  };
}

export function removeNoteFromMeta(meta: Meta, filename: string): Meta {
  return {
    version: 1,
    notes: meta.notes.filter((n) => n.filename !== filename),
    nextIndex: meta.nextIndex
  };
}
