import { useState } from 'react';
import { Github, Monitor, FolderOpen, Check, ShieldCheck } from 'lucide-react';
import { useStore } from '../hooks/store-context';
import { desktop } from '../services/desktop';
import type { Settings } from '../types';
import { Button } from './ui';

export function SettingsView() {
  const { snapshot, busy, run, act } = useStore();
  const [token, setToken] = useState('');
  if (!snapshot) return null;
  const save = (change: Partial<Settings>) => run(() => desktop.settings({ ...snapshot.settings, ...change }));
  return <section className="settings-view"><div className="page-heading"><div><h1>Make yourself at home.</h1><p>A few preferences. Everything else takes care of itself.</p></div></div>
    <section className="settings-section"><h2><Monitor size={18} />General</h2><div className="setting-row"><div><strong>Appearance</strong><p>Match your computer, or choose your own.</p></div><select aria-label="Appearance" disabled={busy} value={snapshot.settings.appearance} onChange={e => void save({ appearance: e.target.value as Settings['appearance'] })}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></div><label className="setting-row"><div><strong>Check for updates when Relay opens</strong><p>No background service or constant polling.</p></div><input type="checkbox" role="switch" checked={snapshot.settings.checkOnOpen} disabled={busy} onChange={e => void save({ checkOnOpen: e.target.checked })} /></label></section>
    <section className="settings-section"><h2><Github size={18} />GitHub</h2><div className="connection-status"><span className="status-dot" /><strong>{snapshot.auth}</strong><span>{snapshot.auth === 'Public access' ? 'Sign in to access private repositories and higher rate limits.' : 'Credentials stay on this computer.'}</span></div><label className="setting-row"><div><strong>Use GitHub CLI sign-in</strong><p>Reuse gh auth login for github.com when available.</p></div><input type="checkbox" role="switch" disabled={busy} checked={snapshot.settings.useGithubCli} onChange={e => void save({ useGithubCli: e.target.checked })} /></label>
      <form className="token-form" onSubmit={e => { e.preventDefault(); void run(async () => { await desktop.token(token); setToken(''); }, 'Token saved to your operating system credential store.'); }}><label htmlFor="github-token">Personal access token</label><p className="help-text">Optional. Fine-grained tokens need Metadata, Contents, and Actions read access for the repositories you use.</p><div className="token-input"><input id="github-token" type="password" autoComplete="off" value={token} onChange={e => setToken(e.target.value)} placeholder="Paste a GitHub token" disabled={busy} /><Button type="submit" disabled={busy || !token.trim()}>Save token</Button></div><button type="button" className="text-button" disabled={busy} onClick={() => void run(() => desktop.token(''), 'Saved token removed. GitHub CLI access follows your preference.')}>Remove saved token</button></form>
    </section>
    <section className="settings-section"><h2><FolderOpen size={18} />Storage</h2><div className="setting-row"><div><strong>Application library</strong><p className="path-text">{snapshot.root}</p></div><Button disabled={busy} onClick={() => void act({ type: 'reveal', id: null, data: false })}>Open folder</Button></div><div className="settings-note"><ShieldCheck size={18} /><p>Packages are staged before activation. Relay keeps the previous version for recovery and never deletes app data outside folders it can identify.</p></div></section>
    <footer className="settings-footer"><span>Relay 0.1.0</span><span><Check size={13} />No AI services. No analytics.</span></footer>
  </section>;
}
