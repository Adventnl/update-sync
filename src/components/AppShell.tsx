import { useEffect, useState } from 'react';
import { Plus, RefreshCw, X, AlertCircle, ArrowLeft, Settings, HardDrive } from 'lucide-react';
import * as Dropdown from '@radix-ui/react-dropdown-menu';
import { Library } from './Library';
import { AppDetails } from './AppDetails';
import { InstallDialog } from './InstallDialog';
import { StorageView } from './StorageView';
import { SettingsView } from './SettingsView';
import { Button, EmptyState } from './ui';
import { useStore } from '../hooks/store-context';
import { desktop } from '../services/desktop';
import { bytes } from '../lib/format';
import type { AppRecord, Prepared } from '../types';

export function AppShell() {
  const { snapshot, loading, error, busy, progress, notices, dismiss, act, run, refresh } = useStore();
  const [view, setView] = useState<'library' | 'storage' | 'settings'>('library');
  const [selected, setSelected] = useState<string | null>(null);
  const [install, setInstall] = useState<{ app?: AppRecord; prepared?: Prepared } | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const openAdd = () => setInstall({});
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n' && !busy) { e.preventDefault(); setInstall({}); }
    };
    window.addEventListener('keydown', handler); return () => window.removeEventListener('keydown', handler);
  }, [busy]);
  const selectedApp = snapshot?.apps.find(a => a.id === selected);
  return <div className="app-shell">
    <main className="main">
      <header className="utility-header">
        <div className="window-heading">{view !== 'library' && <button className="icon-button" aria-label="Back to apps" onClick={() => setView('library')}><ArrowLeft size={18} /></button>}<h1>{view === 'library' ? 'Relay' : view === 'storage' ? 'Storage' : 'Settings'}</h1></div>
        <div className="utility-actions">
          <button className="icon-button" aria-label="Check for updates" title="Check for updates" disabled={busy || !snapshot} onClick={() => void act({ type: 'checkUpdates' }, 'Checked for updates.')}><RefreshCw size={16} className={progress?.stage === 'checking' ? 'spin' : ''} /></button>
          <Dropdown.Root open={menuOpen} onOpenChange={setMenuOpen}><Dropdown.Trigger asChild><button className="icon-button" aria-label="Settings and storage" title="Settings and storage" onPointerDown={event => { if (event.button === 0 && !event.ctrlKey) event.preventDefault(); }} onClick={() => setMenuOpen(open => !open)}><Settings size={17} /></button></Dropdown.Trigger><Dropdown.Portal><Dropdown.Content className="context-menu" align="end" sideOffset={8}><Dropdown.Item onSelect={() => { setView('settings'); setMenuOpen(false); }}>Settings<Settings size={14} /></Dropdown.Item><Dropdown.Item onSelect={() => { setView('storage'); setMenuOpen(false); }}>Storage<HardDrive size={14} /></Dropdown.Item></Dropdown.Content></Dropdown.Portal></Dropdown.Root>
          <Button variant="primary" onClick={openAdd} disabled={busy || !snapshot}><Plus size={16} />Add app</Button>
        </div>
      </header>
      <div className="content">
        {loading ? <div className="loading-library" aria-label="Loading library"><div /><div /><div /></div> : error ? <EmptyState title="Relay needs its desktop connection" icon={<AlertCircle size={30} />} action={<Button onClick={() => void run(refresh)}>Try again</Button>}>{error}</EmptyState> : <>
          {view === 'library' && <Library onSelect={app => setSelected(app.id)} onInstall={app => setInstall({ app })} />}
          {view === 'storage' && <StorageView onResume={plan => setInstall({ app: snapshot?.apps.find(a => a.id === plan.appId), prepared: plan })} />}{view === 'settings' && <SettingsView />}
        </>}
      </div>
      {busy && <div className="operation-strip" role="status" aria-live="polite"><div className="operation-copy"><span className="operation-indicator" /><div><strong>{progress?.message ?? 'Working…'}</strong><span>{progress?.total ? `${bytes(progress.completed)} of ${bytes(progress.total)}` : null}</span></div></div>
        {progress?.total ? <progress max={progress.total} value={progress.completed} aria-label="Download progress" /> : <progress aria-label="Operation progress" />}
        <Button variant="ghost" onClick={() => void desktop.cancel()}>Cancel</Button>
      </div>}
    </main>
    <div className="notices" aria-live="polite">{notices.map(n => <div key={n.id} className={`notice ${n.error ? 'error' : ''}`} role={n.error ? 'alert' : 'status'}><span>{n.text}</span><button className="icon-button" aria-label="Dismiss notification" onClick={() => dismiss(n.id)}><X size={15} /></button></div>)}</div>
    {selectedApp && <AppDetails app={selectedApp} onClose={() => setSelected(null)} onInstall={() => setInstall({ app: selectedApp })} />}
    {install && <InstallDialog app={install.app} initialPrepared={install.prepared} onClose={() => setInstall(null)} />}
  </div>;
}
