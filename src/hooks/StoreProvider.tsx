import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { StoreContext, type Notice } from './store-context';
import { desktop } from '../services/desktop';
import { message } from '../lib/format';
import type { Action, Progress, Snapshot } from '../types';

export function StoreProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [notices, setNotices] = useState<Notice[]>([]);
  const locked = useRef(false);
  const mounted = useRef(true);
  const autoChecked = useRef(false);
  const sequence = useRef(0);
  const notify = useCallback((text: string, error = false) => {
    if (mounted.current) setNotices(old => [...old.slice(-3), { id: ++sequence.current, text, error }]);
  }, []);
  const refresh = useCallback(async () => {
    const value = await desktop.snapshot();
    if (mounted.current) { setSnapshot(value); setError(null); }
  }, []);
  const run = useCallback(async <T,>(task: () => Promise<T>, success?: string): Promise<T | undefined> => {
    if (locked.current) { notify('Wait for the current operation to finish.'); return undefined; }
    locked.current = true; setBusy(true); setProgress(null);
    try {
      const value = await task();
      await refresh();
      if (success) notify(success);
      return value;
    } catch (e) {
      notify(message(e), true);
      try { await refresh(); } catch (refreshError) { notify(`The library could not be refreshed: ${message(refreshError)}`, true); }
      return undefined;
    }
    finally { locked.current = false; if (mounted.current) { setBusy(false); setProgress(null); } }
  }, [notify, refresh]);
  const act = useCallback(async (action: Action, success?: string) => {
    if (action.type === 'launch' || action.type === 'repository' || action.type === 'reveal') {
      try { await desktop.perform(action); if (success) notify(success); }
      catch (e) { notify(message(e), true); }
    } else await run(() => desktop.perform(action), success);
  }, [run, notify]);
  useEffect(() => {
    mounted.current = true;
    let unlisten: (() => void) | undefined;
    desktop.listen(p => { if (mounted.current) setProgress(p); }).then(fn => { if (mounted.current) unlisten = fn; else fn(); }).catch(e => notify(message(e), true));
    refresh().catch(e => { if (mounted.current) setError(message(e)); }).finally(() => { if (mounted.current) setLoading(false); });
    return () => { mounted.current = false; unlisten?.(); };
  }, [refresh, notify]);
  useEffect(() => {
    if (!snapshot || autoChecked.current) return;
    autoChecked.current = true;
    if (snapshot.settings.checkOnOpen && snapshot.apps.some(a => a.tracked)) void act({ type: 'checkUpdates' });
  }, [snapshot, act]);
  useEffect(() => {
    const appearance = snapshot?.settings.appearance ?? 'system';
    document.documentElement.dataset.theme = appearance;
    const media = window.matchMedia?.('(prefers-color-scheme: dark)');
    const sync = () => {
      const theme = appearance === 'system' ? media?.matches ? 'dark' : 'light' : appearance;
      void desktop.appearance(theme).catch(e => notify(`Window appearance could not be updated: ${message(e)}`, true));
    };
    sync();
    if (appearance === 'system') media?.addEventListener('change', sync);
    return () => media?.removeEventListener('change', sync);
  }, [snapshot?.settings.appearance, notify]);
  return <StoreContext.Provider value={{ snapshot, loading, error, busy, progress, notices, refresh, notify, run, act, dismiss: id => setNotices(old => old.filter(n => n.id !== id)) }}>{children}</StoreContext.Provider>;
}
