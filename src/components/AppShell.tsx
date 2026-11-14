import { useEffect, useRef, useState } from 'react';
import { Plus, RefreshCw, Search, X, AlertCircle } from 'lucide-react';
import { Sidebar } from './Sidebar';
import { Library } from './Library';
import { AppDetails } from './AppDetails';
import { InstallDialog } from './InstallDialog';
import { StorageView } from './StorageView';
import { SettingsView } from './SettingsView';
import { ActivityView } from './ActivityView';
import { Button, EmptyState } from './ui';
import { useStore } from '../hooks/store-context';
import { desktop } from '../services/desktop';
import { bytes } from '../lib/format';
import type { AppRecord, Prepared, View } from '../types';

const titles: Record<View, string> = { library: 'Library', updates: 'Updates', storage: 'Storage', activity: 'Activity', settings: 'Settings', untracked: 'Not tracking' };
export function AppShell() {
  const { snapshot, loading, error, busy, progress, notices, dismiss, act, run, refresh } = useStore();
  const [view, setView] = useState<View>('library');
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [install, setInstall] = useState<{ app?: AppRecord; prepared?: Prepared } | null>(null);
  const search = useRef<HTMLInputElement>(null);
  const openAdd = () => setInstall({});
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') { e.preventDefault(); search.current?.focus(); }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n' && !busy) { e.preventDefault(); setInstall({}); }
    };
    window.addEventListener('keydown', handler); return () => window.removeEventListener('keydown', handler);
  }, [busy]);
  const selectedApp = snapshot?.apps.find(a => a.id === selected);
  return <div className="app-shell">
    <Sidebar view={view} onNavigate={next => { setView(next); setQuery(''); }} onAdd={openAdd} />
    <main className="main">
      <header className="toolbar"><span className="toolbar-location">{titles[view]}</span><div className="toolbar-actions">
        <label className="search"><Search size={15} /><input ref={search} type="search" aria-label="Search apps" placeholder="Search your apps" value={query} onChange={e => setQuery(e.target.value)} /><kbd>{snapshot?.platform === 'macos' ? '⌘' : 'Ctrl'} F</kbd></label>
        <button className="icon-button" aria-label="Check for updates" title="Check for updates" disabled={busy || !snapshot} onClick={() => void act({ type: 'checkUpdates' }, 'Your apps have been checked.')}><RefreshCw size={17} className={progress?.stage === 'checking' ? 'spin' : ''} /></button>
        <Button variant="primary" onClick={openAdd} disabled={busy || !snapshot}><Plus size={16} />Add app</Button>
      </div></header>
      <div className="content">
        {loading ? <div className="loading-library" aria-label="Loading library"><div /><div /><div /></div> : error ? <EmptyState title="Relay needs its desktop connection" icon={<AlertCircle size={30} />} action={<Button onClick={() => void run(refresh)}>Try again</Button>}>{error}</EmptyState> : <>
          {(view === 'library' || view === 'updates' || view === 'untracked') && <Library view={view} query={query} onSelect={app => setSelected(app.id)} onInstall={app => setInstall({ app })} onAdd={openAdd} />}
          {view === 'storage' && <StorageView onResume={plan => setInstall({ app: snapshot?.apps.find(a => a.id === plan.appId), prepared: plan })} />}{view === 'settings' && <SettingsView />}{view === 'activity' && <ActivityView query={query} />}
        </>}
      </div>
      {busy && <div className="operation-strip" role="status" aria-live="polite"><div className="operation-copy"><span className="operation-indicator" /><div><strong>{progress?.message ?? 'Working…'}</strong><span>{progress?.total ? `${bytes(progress.completed)} of ${bytes(progress.total)}` : 'Your current installation stays available until this finishes.'}</span></div></div>
        {progress?.total ? <progress max={progress.total} value={progress.completed} aria-label="Download progress" /> : <progress aria-label="Operation progress" />}
        <Button variant="ghost" onClick={() => void desktop.cancel()}>Cancel</Button>
      </div>}
    </main>
    <div className="notices" aria-live="polite">{notices.map(n => <div key={n.id} className={`notice ${n.error ? 'error' : ''}`} role={n.error ? 'alert' : 'status'}><span>{n.text}</span><button className="icon-button" aria-label="Dismiss notification" onClick={() => dismiss(n.id)}><X size={15} /></button></div>)}</div>
    {selectedApp && <AppDetails app={selectedApp} onClose={() => setSelected(null)} onInstall={() => setInstall({ app: selectedApp })} />}
    {install && <InstallDialog app={install.app} initialPrepared={install.prepared} onClose={() => setInstall(null)} />}
  </div>;
}
