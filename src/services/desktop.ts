import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { Action, AppRecord, BuildConfig, BuildPlan, Channel, Inspection, Prepared, Progress, Settings, Snapshot, StorageItem, Workflow } from '../types';

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return Promise.reject(new Error('Open Relay as a desktop app. Run npm run desktop to connect to the native application manager.'));
  return invoke<T>(command, args);
}
export const desktop = {
  snapshot: () => call<Snapshot>('snapshot'),
  inspect: (url: string, channel: Channel) => call<Inspection>('inspect_repository', { url, channel }),
  add: (url: string, channel: Channel) => call<AppRecord>('add_app', { url, channel }),
  perform: (action: Action) => call<void>('perform', { action }),
  prepare: (id: string, packageId: string, allowUnverified: boolean) => call<Prepared>('prepare_install', { id, packageId, allowUnverified }),
  commit: (ticket: string, entry: string) => call<AppRecord>('commit_install', { ticket, entry }),
  buildPlan: (id: string) => call<BuildPlan>('build_plan', { id }),
  build: (id: string) => call<Prepared>('local_build', { id, trust: true }),
  buildSource: (id: string, configuration: BuildConfig | null = null) => call<Prepared>('build_source', { id, configuration }),
  workflows: (id: string) => call<Workflow[]>('workflows', { id }),
  requestBuild: (id: string, workflow: number) => call<void>('request_build', { id, workflow }),
  storage: () => call<StorageItem[]>('storage_items'),
  prepared: () => call<Prepared[]>('prepared_installs'),
  cleanup: (ids: string[]) => call<number>('cleanup', { ids }),
  scan: (root: string) => call<StorageItem[]>('scan_caches', { root }),
  removeCache: (root: string, path: string) => call<number>('remove_cache', { root, path }),
  settings: (settings: Settings) => call<void>('save_settings', { settings }),
  token: (token: string) => call<void>('save_token', { token }),
  verify: (id: string) => call<string[]>('verify_install', { id }),
  registerData: (id: string, path: string) => call<void>('register_data', { id, path }),
  forgetData: (id: string, path: string) => call<void>('forget_data', { id, path }),
  cancel: () => call<void>('cancel_operation'),
  listen: (callback: (progress: Progress) => void) => isTauri() ? listen<Progress>('operation-progress', e => callback(e.payload)) : Promise.resolve(() => {}),
  folder: (title = 'Choose your development projects folder') => open({ directory: true, multiple: false, title }),
};
