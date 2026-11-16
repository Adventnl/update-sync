import { useCallback, useEffect, useState } from 'react';
import { Archive, FolderSearch, Trash2, RefreshCw } from 'lucide-react';
import { useStore } from '../hooks/store-context';
import { desktop } from '../services/desktop';
import { bytes, message } from '../lib/format';
import type { Prepared, StorageItem } from '../types';
import { Button, Modal } from './ui';

export function StorageView({ onResume }: { onResume: (prepared: Prepared) => void }) {
  const { busy, run, notify, snapshot } = useStore();
  const [items, setItems] = useState<StorageItem[]>([]);
  const [cacheResult, setCacheResult] = useState<{ root: string; items: StorageItem[] }>({ root: '', items: [] });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [managedError, setManagedError] = useState<string | null>(null);
  const [preparedError, setPreparedError] = useState<string | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [confirm, setConfirm] = useState(false);
  const [pending, setPending] = useState<Prepared[]>([]);
  useEffect(() => { let live = true; desktop.prepared().then(plans => { if (live) { setPending(plans); setPreparedError(null); } }).catch(e => { if (live) setPreparedError(message(e)); }); return () => { live = false; }; }, [snapshot]);
  const load = useCallback(async () => {
    setLoading(true);
    try { const list = await desktop.storage(); setItems(list); setManagedError(null); }
    catch (e) { setManagedError(message(e)); }
    finally { setLoading(false); }
  }, []);
  useEffect(() => { let live = true; setLoading(true); desktop.storage().then(list => { if (live) { setItems(list); setManagedError(null); } }).catch(e => { if (live) setManagedError(message(e)); }).finally(() => { if (live) setLoading(false); }); return () => { live = false; }; }, [snapshot]);
  const all = [...(managedError ? [] : items), ...cacheResult.items];
  const chosen = all.filter(i => selected.has(i.id));
  const total = chosen.reduce((n, i) => n + i.size, 0);
  const installed = snapshot?.apps.reduce((n, a) => n + a.versions.reduce((s, v) => s + v.size, 0), 0) ?? 0;
  const scan = async () => {
    try {
      const folder = await desktop.folder();
      if (typeof folder !== 'string') return;
      await run(async () => {
        try { const found = await desktop.scan(folder); setCacheResult({ root: folder, items: found }); setScanError(null); notify('Scan complete. Only recognized caches older than a day are listed.'); }
        catch (e) { setScanError(message(e)); throw e; }
      });
    } catch (e) { setScanError(message(e)); notify(message(e), true); }
  };
  const clean = async () => {
    await run(async () => {
      try {
        let freed = 0;
        const owned = chosen.filter(i => i.kind !== 'cache').map(i => i.id);
        if (owned.length) freed += await desktop.cleanup(owned);
        for (const cache of chosen.filter(i => i.kind === 'cache')) {
          freed += await desktop.removeCache(cacheResult.root, cache.path);
          setCacheResult(old => ({ ...old, items: old.items.filter(i => i.id !== cache.id) }));
        }
        setConfirm(false); setSelected(new Set()); notify(`${bytes(freed)} of storage reclaimed.`);
      } finally { await load(); }
    });
  };
  return <section className="storage-view">
    <div className="storage-summary"><span>Managed versions</span><strong>{bytes(installed)}</strong><button className="icon-button" aria-label="Refresh storage" title="Refresh storage" disabled={busy || loading} onClick={() => void run(load)}><RefreshCw size={15} /></button></div>
    {preparedError ? <div className="inline-error" role="alert">Prepared downloads are unavailable. {preparedError}</div> : pending.length > 0 && <section className="pending-downloads"><h2>Ready to finish</h2>{pending.map(plan => <div key={plan.ticket}><span><strong>{snapshot?.apps.find(a => a.id === plan.appId)?.name ?? 'Application'}</strong><small>{plan.package.version} · Download already prepared</small></span><Button disabled={busy} onClick={() => onResume(plan)}>Resume installation</Button></div>)}</section>}
    {(all.length > 0 || managedError || loading) && <div className="section-heading"><h2>Cleanup</h2><span>{managedError ? 'Storage unavailable' : loading ? 'Measuring…' : bytes(all.reduce((n, i) => n + i.size, 0))}</span></div>}
    {managedError && <div className="inline-error" role="alert">Managed storage could not be measured. {managedError}<div><Button disabled={busy} onClick={() => void run(load)}>Try again</Button></div></div>}
    {loading ? <p className="muted">Measuring storage…</p> : all.length ? <div className="storage-list">{all.map(item => <label className="storage-row" key={item.id}><input type="checkbox" checked={selected.has(item.id)} disabled={busy} onChange={e => setSelected(old => { const next = new Set(old); if (e.target.checked) next.add(item.id); else next.delete(item.id); return next; })} /><Archive size={21} className="muted" /><span><strong>{item.name}</strong><small>{item.detail}</small><span className="storage-path">{item.path}</span></span><b>{bytes(item.size)}</b></label>)}</div> : !managedError && <p className="clean-state">Nothing to clean up</p>}
    {all.length > 0 && <div className="cleanup-bar"><span>{chosen.length} selected · {bytes(total)}</span><Button variant="primary" disabled={busy || !chosen.length} onClick={() => setConfirm(true)}><Trash2 size={15} />Clean up</Button></div>}
    <div className="cache-section"><div><h2>Development caches</h2><p>Remove old build caches from a folder you choose.</p></div><Button disabled={busy} onClick={() => void scan()}><FolderSearch size={16} />Choose folder…</Button>{cacheResult.root && <p className="help-text path-text">Scanned {cacheResult.root}</p>}{scanError && <p className="inline-error" role="alert">The cache scan did not finish. {scanError}{cacheResult.root && ' Previous results are kept for the folder shown above.'}</p>}</div>

    <Modal open={confirm} onOpenChange={setConfirm} title={`Remove ${bytes(total)} of files?`} description="These files are permanently removed. Development caches will be recreated the next time you build." busy={busy}><ul className="cleanup-confirm-list">{chosen.map(i => <li key={i.id}><span>{i.name}</span><span>{bytes(i.size)}</span></li>)}</ul><div className="dialog-footer"><Button disabled={busy} onClick={() => setConfirm(false)}>Keep files</Button><Button variant="danger" disabled={busy} onClick={() => void clean()}>Remove selected files</Button></div></Modal>
  </section>;
}
