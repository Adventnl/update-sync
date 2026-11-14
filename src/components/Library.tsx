import * as Dropdown from '@radix-ui/react-dropdown-menu';
import * as ContextMenu from '@radix-ui/react-context-menu';
import { useState } from 'react';
import type { ComponentType, ReactNode } from 'react';
import { ArrowUpRight, Check, ChevronRight, Download, ExternalLink, FolderOpen, MoreHorizontal, PackagePlus, Plus, Unplug, ShieldCheck, RefreshCw } from 'lucide-react';
import type { AppRecord, View } from '../types';
import { useStore } from '../hooks/store-context';
import { active, hasUpdate } from '../lib/format';
import { AppIcon, Button, EmptyState } from './ui';

export function Library({ view, query, onSelect, onInstall, onAdd }: { view: View; query: string; onSelect: (app: AppRecord) => void; onInstall: (app: AppRecord) => void; onAdd: () => void }) {
  const { snapshot, busy, act } = useStore();
  const [sort, setSort] = useState('name');
  const all = snapshot?.apps ?? [];
  const apps = all.filter(a => view === 'untracked' ? !a.tracked : a.tracked && (view !== 'updates' || hasUpdate(a))).filter(a => `${a.name} ${a.repository.owner} ${a.description}`.toLowerCase().includes(query.toLowerCase())).sort((a, b) => sort === 'name' ? a.name.localeCompare(b.name) : (b.versions[0]?.installedAt ?? '').localeCompare(a.versions[0]?.installedAt ?? ''));
  const isUpdates = view === 'updates';
  const installed = all.filter(a => a.tracked && active(a)).length;
  return <section className="library-view">
    <div className="page-heading"><div><h1>{isUpdates ? 'Updates' : view === 'untracked' ? 'Not tracking' : 'Your library'}</h1><p>{isUpdates ? 'The next version, without the busywork.' : view === 'untracked' ? 'These apps stay installed. Reconnect whenever you like.' : installed ? `${installed} ${installed === 1 ? 'app' : 'apps'} installed. All in one place.` : 'A home for the apps you make.'}</p></div>
      {apps.length > 1 && <label className="sort-control"><span className="sr-only">Sort apps</span><select value={sort} onChange={e => setSort(e.target.value)}><option value="name">Name</option><option value="recent">Recently installed</option></select></label>}
    </div>
    {apps.length > 0 ? <><div className="section-label"><span>{isUpdates ? 'Ready to update' : view === 'untracked' ? 'Disconnected apps' : 'All apps'}</span><span>{apps.length}</span></div><div className="app-list">{apps.map(app => <AppRow key={app.id} app={app} busy={busy} onSelect={() => onSelect(app)} onInstall={() => onInstall(app)} />)}</div>
      {!isUpdates && view !== 'untracked' && <button className="add-row" onClick={onAdd} disabled={busy}><div className="add-row-icon"><Plus size={22} /></div><span><strong>Add another app</strong><small>Just paste a GitHub link.</small></span><ChevronRight size={17} /></button>}
    </> : query ? <EmptyState title="No matching apps">Try another name or repository owner.</EmptyState> : isUpdates ? <EmptyState title="You're up to date" icon={<Check size={30} />} action={<Button disabled={busy} onClick={() => void act({ type: 'checkUpdates' }, 'Your apps have been checked.')}><RefreshCw size={15} />Check again</Button>}>Successful builds and stable releases will appear here when they're ready.</EmptyState> : view === 'untracked' ? <EmptyState title="Everything is connected" icon={<Unplug size={30} />}>Apps you stop tracking will appear here. Their files and shortcuts will stay where they are.</EmptyState> : <div className="welcome"><div className="welcome-symbol"><PackagePlus size={42} strokeWidth={1.25} /></div><h2>Your next app<br />starts with a link.</h2><p>Bring your GitHub projects together.<br />Install them once. Keep them up to date.</p><Button variant="primary" onClick={onAdd}><Plus size={17} />Add your first app</Button><div className="welcome-note"><ShieldCheck size={14} />Local to your computer. Connected to GitHub.</div></div>}
  </section>;
}

function AppRow({ app, busy, onSelect, onInstall }: { app: AppRecord; busy: boolean; onSelect: () => void; onInstall: () => void }) {
  const { act, progress } = useStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const version = active(app); const update = hasUpdate(app);
  const launchBlocked = busy && progress?.stage !== 'checking';
  const actions = [
    ...(version ? [{ label: 'Open', icon: ArrowUpRight, action: () => void act({ type: 'launch', id: app.id }), disabled: launchBlocked }] : []),
    { label: 'App details', icon: ChevronRight, action: onSelect },
    { label: 'Install or repair', icon: Download, action: onInstall, disabled: busy },
    { label: 'Show in folder', icon: FolderOpen, action: () => void act({ type: 'reveal', id: app.id, data: false }), disabled: !version || busy },
    { label: 'View on GitHub', icon: ExternalLink, action: () => void act({ type: 'repository', id: app.id }), disabled: busy },
    { label: app.tracked ? 'Stop tracking' : 'Resume tracking', icon: Unplug, action: () => void act({ type: 'track', id: app.id, tracked: !app.tracked }, app.tracked ? 'Tracking stopped. App files and shortcuts are unchanged.' : 'Tracking resumed.'), disabled: busy },
  ];
  const items = (Item: ComponentType<{ children: ReactNode; disabled?: boolean; onSelect?: () => void }>) => actions.map(({ label, icon: Icon, action, disabled }) => <Item key={label} disabled={disabled} onSelect={action}>{label}<Icon size={14} /></Item>);
  return <ContextMenu.Root><ContextMenu.Trigger asChild><div className="app-row">
    <button className="app-identity" onClick={onSelect} aria-label={`View ${app.name}`}><AppIcon name={app.name} /><span><strong>{app.name}</strong><span className="app-repository">{app.repository.owner} / {app.repository.name}</span></span></button>
    <div className="app-status">{update ? <span className="update-status"><span />Update available</span> : !app.tracked ? <span>Not tracking</span> : app.checkError ? <span className="warning-text">Couldn't check</span> : version ? <span>{version.label}</span> : <span>Not installed</span>}<small>{app.channel === 'main' ? 'Successful main builds' : 'Stable releases'}</small></div>
    <div className="app-row-actions">{update && <button className="icon-button" aria-label={`Open ${app.name}`} title={`Open ${app.name}`} disabled={launchBlocked} onClick={() => void act({ type: 'launch', id: app.id })}><ArrowUpRight size={17} /></button>}<Button className={update || !version ? '' : 'open-button'} variant={update || !version ? 'primary' : 'secondary'} disabled={version && !update ? launchBlocked : busy} onClick={() => version && !update ? void act({ type: 'launch', id: app.id }) : onInstall()}>{update ? <><Download size={14} />Update</> : version ? <>Open<ArrowUpRight size={13} /></> : <><Download size={14} />Install</>}</Button>
      <Dropdown.Root open={menuOpen} onOpenChange={setMenuOpen}><Dropdown.Trigger asChild><button className="icon-button" aria-label={`Actions for ${app.name}`} onPointerDown={event => {
        // Use click for both pointer and Windows accessibility activation.
        if (event.button === 0 && !event.ctrlKey) event.preventDefault();
      }} onClick={() => setMenuOpen(open => !open)}><MoreHorizontal size={19} /></button></Dropdown.Trigger><Dropdown.Portal><Dropdown.Content className="context-menu" sideOffset={5} align="end">{items(Dropdown.Item)}</Dropdown.Content></Dropdown.Portal></Dropdown.Root>
    </div>
  </div></ContextMenu.Trigger><ContextMenu.Portal><ContextMenu.Content className="context-menu">{items(ContextMenu.Item)}</ContextMenu.Content></ContextMenu.Portal></ContextMenu.Root>;
}
