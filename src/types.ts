export type Channel = 'stable' | 'main';
export interface Repository { owner: string; name: string }
export interface Package {
  id: string; name: string; url: string; version: string; commit: string | null;
  digest: string | null; size: number; source: string; publishedAt: string; compatible: boolean;
}
export interface Inspection {
  repository: Repository; repositoryId: number; description: string; defaultBranch: string;
  headCommit: string | null; stacks: string[]; packages: Package[]; warnings: string[]; htmlUrl: string;
}
export interface InstalledVersion {
  id: string; label: string; commit: string | null; installedAt: string; entry: string;
  packageDigest: string; source: string; size: number;
}
export interface AppRecord {
  id: string; repositoryId: number; repository: Repository; name: string; description: string;
  stacks: string[]; channel: Channel; tracked: boolean; activeVersion: string | null;
  versions: InstalledVersion[]; available: Package | null; checkedAt: string | null;
  checkError: string | null; desktopShortcut: boolean;
  dataFolders: string[];
  buildConfig?: BuildConfig | null;
}
export interface Activity { id: string; appId: string; appName: string; action: string; detail: string; at: string }
export interface Settings { appearance: 'system' | 'light' | 'dark'; useGithubCli: boolean; checkOnOpen: boolean }
export interface Snapshot {
  apps: AppRecord[]; activity: Activity[]; settings: Settings; root: string;
  platform: string; architecture: string; auth: string;
}
export interface Progress { appId: string; stage: string; message: string; completed: number; total: number | null }
export interface Prepared { ticket: string; appId: string; package: Package; digest: string; entries: string[]; previous: string | null }
export interface StorageItem { id: string; name: string; path: string; size: number; kind: string; appId: string | null; detail: string }
export interface BuildPlan { stack: string; steps: string[]; warning: string }
export interface BuildConfig {
  system: 'app' | 'tauri' | 'electron' | 'cargo' | 'cmake' | 'maven' | 'gradle' | 'python' | 'go' | 'dotnet';
  project: string; target: string | null; output: string | null; entry: string | null; resources: string[];
}
export interface Workflow { id: number; name: string; path: string; state: string }
export type Action =
  | { type: 'checkUpdates' }
  | { type: 'launch' | 'repository'; id: string }
  | { type: 'track'; id: string; tracked: boolean }
  | { type: 'channel'; id: string; channel: Channel }
  | { type: 'shortcuts'; id: string; desktop: boolean }
  | { type: 'rollback'; id: string; version: string }
  | { type: 'uninstall'; id: string; removeData: boolean }
  | { type: 'reveal'; id: string | null; data: boolean }
  | { type: 'discard'; ticket: string };
