import { invoke } from '@tauri-apps/api/core';

export interface MdFileInfo {
  filename: string;
  mtime_ms: number;
}

export async function getDefaultStorageFolder(): Promise<string> {
  return invoke<string>('get_default_storage_folder');
}

export async function pathExists(path: string): Promise<boolean> {
  return invoke<boolean>('path_exists', { path });
}

export async function createDir(path: string): Promise<void> {
  await invoke('create_dir', { path });
}

// One-time handshake: the renderer resolves & creates the user's chosen
// folder, then hands it to Rust. Every storage command below reads from
// Rust state — the folder is never passed again. Must be called before
// any of {listMarkdownFiles, readNote, writeNote, ...}.
export async function setStorageFolder(folder: string): Promise<void> {
  await invoke('set_storage_folder', { folder });
}

export async function listMarkdownFiles(): Promise<MdFileInfo[]> {
  return invoke<MdFileInfo[]>('list_md_files');
}

export async function readNote(filename: string): Promise<string> {
  return invoke<string>('read_note', { filename });
}

export async function writeNote(filename: string, contents: string): Promise<void> {
  await invoke('write_note', { filename, contents });
}

export async function deleteNote(filename: string): Promise<void> {
  await invoke('delete_note', { filename });
}

export async function getMtime(filename: string): Promise<number> {
  return invoke<number>('get_mtime', { filename });
}

export async function readAppConfig(): Promise<string | null> {
  return invoke<string | null>('read_app_config');
}

export async function writeAppConfig(json: string): Promise<void> {
  await invoke('write_app_config', { json });
}
