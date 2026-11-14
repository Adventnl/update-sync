import { useEffect, useState } from 'react';
import { ArrowRight, Download, Github, ShieldCheck, Terminal, ExternalLink, CheckCircle2, Play } from 'lucide-react';
import { AppIcon, Button, Feedback, Modal } from './ui';
import { desktop } from '../services/desktop';
import { useStore } from '../hooks/store-context';
import { bytes, message } from '../lib/format';
import type { AppRecord, BuildPlan, Channel, Inspection, Prepared, Workflow } from '../types';

export function InstallDialog({ app, initialPrepared, onClose }: { app?: AppRecord; initialPrepared?: Prepared; onClose: () => void }) {
  const { busy, progress, run, notify } = useStore();
  const [url, setUrl] = useState(app ? `https://github.com/${app.repository.owner}/${app.repository.name}` : '');
  const [channel, setChannel] = useState<Channel>(app?.channel ?? 'stable');
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const [selected, setSelected] = useState('');
  const [allowUnverified, setAllowUnverified] = useState(false);
  const [prepared, setPrepared] = useState<Prepared | null>(initialPrepared ?? null);
  const [entry, setEntry] = useState(initialPrepared?.entries[0] ?? '');
  const [build, setBuild] = useState<BuildPlan | null>(null);
  const [trustBuild, setTrustBuild] = useState(false);
  const [loading, setLoading] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [record, setRecord] = useState(app);
  const [workflows, setWorkflows] = useState<Workflow[] | null>(null);
  const [workflow, setWorkflow] = useState('');
  const clearReview = () => {
    setInspection(null); setSelected(''); setAllowUnverified(false);
    setRecord(app); setBuild(null); setTrustBuild(false); setWorkflows(null); setWorkflow('');
    setLocalError(null);
  };
  useEffect(() => {
    if (!app || initialPrepared) return;
    let current = true; setLoading(true);
    desktop.inspect(`https://github.com/${app.repository.owner}/${app.repository.name}`, app.channel).then(info => {
      if (current) { setInspection(info); setSelected(info.packages.find(p => p.compatible)?.id ?? ''); }
    }).catch(e => { if (current) setLocalError(message(e)); }).finally(() => { if (current) setLoading(false); });
    return () => { current = false; };
  }, [app, initialPrepared]);
  const inspect = async () => {
    clearReview(); setLoading(true);
    try { const info = await desktop.inspect(url, channel); setInspection(info); setSelected(info.packages.find(p => p.compatible)?.id ?? ''); }
    catch (e) { setLocalError(message(e)); }
    finally { setLoading(false); }
  };
  const packageInfo = inspection?.packages.find(p => p.id === selected);
  const finish = async (plan: Prepared) => {
    setPrepared(plan); setEntry(plan.entries[0] ?? '');
    if (plan.entries.length === 1) {
      await desktop.commit(plan.ticket, plan.entries[0]);
      notify('Installed and ready to open.'); onClose();
    }
  };
  const install = async () => {
    await run(async () => {
      const current = record?.repositoryId === inspection?.repositoryId && record?.channel === channel ? record : await desktop.add(url, channel); setRecord(current);
      if (!current) throw new Error('Find the repository again before installing.');
      const plan = await desktop.prepare(current.id, selected, allowUnverified);
      await finish(plan);
    });
  };
  const reviewBuild = async () => {
    await run(async () => {
      const current = record?.repositoryId === inspection?.repositoryId && record?.channel === channel ? record : await desktop.add(url, channel); setRecord(current);
      if (!current) throw new Error('Find the repository again before building.');
      setBuild(await desktop.buildPlan(current.id));
    });
  };
  const reviewRemote = async () => {
    await run(async () => {
      const current = record?.repositoryId === inspection?.repositoryId && record?.channel === channel ? record : await desktop.add(url, channel); setRecord(current);
      if (!current) throw new Error('Find the repository again before building.');
      const list = await desktop.workflows(current.id); setWorkflows(list); setWorkflow(String(list[0]?.id ?? ''));
    });
  };
  const close = () => {
    if (prepared) notify('Download saved. Resume the installation from Storage.');
    onClose();
  };
  return <Modal open onOpenChange={open => { if (!open) close(); }} title={prepared ? 'Choose the application' : app ? `Install ${app.name}` : 'Add an app'} description={prepared ? 'This package contains more than one launch file. Choose the main application.' : 'A GitHub link is all you need to get started.'} busy={busy}>
    {!prepared && <form onSubmit={e => { e.preventDefault(); void inspect(); }} className="repository-form">
      <label htmlFor="repository-url">GitHub repository</label><div className="repository-input"><Github size={18} /><input id="repository-url" type="url" required autoFocus={!app} placeholder="https://github.com/you/your-app" value={url} disabled={!!app || busy || loading} onChange={e => { clearReview(); setUrl(e.target.value); }} /></div>
      <div className="repository-controls"><label className="channel-field">Channel<select value={channel} disabled={!!app || busy || loading} onChange={e => { clearReview(); setChannel(e.target.value as Channel); }}><option value="stable">Stable releases</option><option value="main">Successful main builds</option></select></label><Button type="submit" disabled={busy || loading || !url}>{loading ? 'Finding app…' : inspection ? 'Refresh' : 'Find app'}<ArrowRight size={15} /></Button></div>
    </form>}
    {localError && <Feedback error>{localError}</Feedback>}
    {inspection && !prepared && <div className="install-review"><div className="repository-preview"><AppIcon name={inspection.repository.name} /><div><h3>{inspection.repository.name}</h3><p>{inspection.description || `${inspection.repository.owner}'s application`}</p><div className="stack-labels">{inspection.stacks.join(' · ') || 'Compiled package'}</div></div></div>
      {!build && workflows === null && <><label className="field-label" htmlFor="package-select">Application package</label><select id="package-select" className="wide-select" value={selected} disabled={busy} onChange={e => { setSelected(e.target.value); setAllowUnverified(false); }}><option value="" disabled>Choose a package</option>{inspection.packages.map(p => <option key={p.id} value={p.id} disabled={!p.compatible}>{p.version} · {p.name}{!p.compatible ? ' · Different platform' : ''}</option>)}</select>
        {packageInfo && <div className="package-meta"><span><Download size={14} />{bytes(packageInfo.size)}</span><span><ShieldCheck size={14} />{packageInfo.digest ? 'SHA-256 provided by GitHub' : 'No publisher checksum'}</span></div>}
        {packageInfo && !packageInfo.digest && <label className="check-row warning-check"><input type="checkbox" checked={allowUnverified} onChange={e => setAllowUnverified(e.target.checked)} disabled={busy} /><span>I trust this repository and approve installing a package without a publisher checksum.</span></label>}
        <details className="disclosure"><summary>Package and verification details</summary><p>Relay verifies provided SHA-256 hashes and records every installed file. Windows signatures are checked when present. macOS apps must pass signature and Gatekeeper checks. Only install software you trust.</p>{packageInfo && <code>{packageInfo.url}</code>}{inspection.warnings.map((warning, i) => <p key={i}>{warning}</p>)}</details>
        <div className="build-alternatives"><button className="text-button" disabled={busy} onClick={() => void reviewRemote()}>Build on GitHub…</button><button className="text-button" disabled={busy} onClick={() => void reviewBuild()}>Review local build…</button></div>
        <div className="dialog-footer"><Button variant="ghost" disabled={busy} onClick={close}>Cancel</Button><Button variant="primary" disabled={busy || !packageInfo || (!packageInfo.digest && !allowUnverified)} onClick={() => void install()}><Download size={16} />{app?.activeVersion ? 'Install version' : 'Install app'}</Button></div>
        {!inspection.packages.length && <p className="help-text">No package found. Try the other channel, publish an Actions artifact, or review a local build.</p>}
      </>}
      {workflows !== null && <div className="remote-build"><h3>Build on GitHub</h3><p className="help-text">Run a workflow on the repository's default branch. It needs a workflow_dispatch trigger with default inputs. When it finishes, check the Main builds channel for its package.</p>{workflows.length > 0 ? <select aria-label="Build workflow" className="wide-select" value={workflow} onChange={e => setWorkflow(e.target.value)}>{workflows.map(w => <option key={w.id} value={w.id}>{w.name}</option>)}</select> : <p className="help-text">No active workflows were found. Add a packaging workflow to the repository or use a local build.</p>}<div className="dialog-footer"><Button disabled={busy} onClick={() => setWorkflows(null)}>Back to packages</Button><Button variant="primary" disabled={busy || !workflow || !record} onClick={() => void run(async () => { if (record) await desktop.requestBuild(record.id, Number(workflow)); notify('GitHub build requested. Check for updates when it finishes.'); onClose(); })}><Play size={15} />Start build</Button></div></div>}
      {build && <div className="build-review"><h3>{build.stack} local build</h3><ol>{build.steps.map(step => <li key={step}>{step}</li>)}</ol><p>{build.warning}</p><label className="check-row"><input type="checkbox" checked={trustBuild} onChange={e => setTrustBuild(e.target.checked)} disabled={busy} />I trust this repository and approve running its build code.</label><div className="dialog-footer"><Button disabled={busy} onClick={() => setBuild(null)}>Back to packages</Button><Button variant="primary" disabled={!trustBuild || busy || !record} onClick={() => void run(async () => { if (record) await finish(await desktop.build(record.id)); })}><Terminal size={16} />Build and install</Button></div></div>}
    </div>}
    {prepared && <div className="entry-choice"><CheckCircle2 size={26} className="success-text" /><p>Your package is downloaded and verified.</p><label htmlFor="launch-file">Application to open</label><select id="launch-file" className="wide-select" value={entry} onChange={e => setEntry(e.target.value)}>{prepared.entries.map(path => <option key={path}>{path}</option>)}</select><div className="dialog-footer"><Button disabled={busy} onClick={() => void run(async () => { await desktop.perform({ type: 'discard', ticket: prepared.ticket }); onClose(); })}>Discard download</Button><Button variant="primary" disabled={busy || !entry} onClick={() => void run(async () => { await desktop.commit(prepared.ticket, entry); notify('Installed and ready to open.'); onClose(); })}>Finish installation<ArrowRight size={15} /></Button></div></div>}
    {!inspection && !prepared && !localError && <div className="install-assurance"><ShieldCheck size={17} /><span>Packages come directly from GitHub.<br />Your settings and previous version stay safe.</span><ExternalLink size={14} /></div>}
    {busy && <div className="dialog-operation" role="status" aria-live="polite"><strong>{progress?.message ?? 'Preparing your application…'}</strong>{progress?.total ? <><progress max={progress.total} value={progress.completed} aria-label="Package download" /><span>{bytes(progress.completed)} of {bytes(progress.total)}</span></> : <progress aria-label="Installation progress" />}<Button variant="ghost" onClick={() => void desktop.cancel()}>Cancel operation</Button></div>}
  </Modal>;
}
