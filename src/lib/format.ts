import type { AppRecord } from '../types';
export function bytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0 B';
  const unit = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 3);
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value / 1024 ** unit)} ${['B', 'KB', 'MB', 'GB'][unit]}`;
}
export function date(value: string): string {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? 'Unknown date' : new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit' }).format(parsed);
}
export function active(app: AppRecord) { return app.versions.find(v => v.id === app.activeVersion); }
export function hasUpdate(app: AppRecord) { return !!active(app) && !!app.available && app.available.id !== active(app)?.source; }
export function appHue(name: string): number { return [...name].reduce((n, c) => (n * 31 + c.charCodeAt(0)) % 360, 0); }
export function message(error: unknown): string { return error instanceof Error ? error.message : String(error); }
