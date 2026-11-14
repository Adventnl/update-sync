import { AppWindow, ArrowDownToLine, Archive, Clock3, Settings, Unplug, Plus, Layers } from 'lucide-react';
import type { View } from '../types';
import { useStore } from '../hooks/store-context';
import { hasUpdate } from '../lib/format';

const destinations = [
  { id: 'library', label: 'Library', icon: AppWindow },
  { id: 'updates', label: 'Updates', icon: ArrowDownToLine },
  { id: 'storage', label: 'Storage', icon: Archive },
  { id: 'activity', label: 'Activity', icon: Clock3 },
] as const;
export function Sidebar({ view, onNavigate, onAdd }: { view: View; onNavigate: (view: View) => void; onAdd: () => void }) {
  const { snapshot, busy } = useStore();
  const updates = snapshot?.apps.filter(a => a.tracked && hasUpdate(a)).length ?? 0;
  return <aside className="sidebar">
    <div className="brand"><div className="brand-mark"><Layers size={22} strokeWidth={1.6} /></div><span>Relay</span></div>
    <nav aria-label="Main navigation">{destinations.map(({ id, label, icon: Icon }) => <button key={id} aria-label={label} title={label} className={`nav-item ${view === id ? 'selected' : ''}`} aria-current={view === id ? 'page' : undefined} onClick={() => onNavigate(id)}><Icon size={18} /><span>{label}</span>{id === 'updates' && updates > 0 && <span className="nav-count">{updates}</span>}</button>)}</nav>
    <button className="sidebar-add" disabled={busy} onClick={onAdd}><Plus size={17} />Add app</button>
    <div className="sidebar-bottom"><div className="local-status"><span className="status-dot" />On this computer</div>
      <button aria-label="Not tracking" title="Not tracking" aria-current={view === 'untracked' ? 'page' : undefined} className={`nav-item ${view === 'untracked' ? 'selected' : ''}`} onClick={() => onNavigate('untracked')}><Unplug size={17} /><span>Not tracking</span></button>
      <button aria-label="Settings" title="Settings" aria-current={view === 'settings' ? 'page' : undefined} className={`nav-item ${view === 'settings' ? 'selected' : ''}`} onClick={() => onNavigate('settings')}><Settings size={17} /><span>Settings</span></button>
      <div className="sidebar-footnote">Your apps. Your computer.</div>
    </div>
  </aside>;
}
