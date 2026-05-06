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

export async function listMarkdownFiles(folder: string): Promise<MdFileInfo[]> {
  return invoke<MdFileInfo[]>('list_md_files', { folder });
}

export async function readNote(folder: string, filename: string): Promise<string> {
  return invoke<string>('read_note', { folder, filename });
}

export async function writeNote(folder: string, filename: string, contents: string): Promise<void> {
  await invoke('write_note', { folder, filename, contents });
}

export async function deleteNote(folder: string, filename: string): Promise<void> {
  await invoke('delete_note', { folder, filename });
}

export async function getMtime(folder: string, filename: string): Promise<number> {
  return invoke<number>('get_mtime', { folder, filename });
}

export async function readAppConfig(): Promise<string | null> {
  return invoke<string | null>('read_app_config');
}

export async function writeAppConfig(json: string): Promise<void> {
  await invoke('write_app_config', { json });
}
