import { useCallback, useEffect, useRef, useState } from 'react';
import { ArrowRight, Download, Github, ShieldCheck, CheckCircle2 } from 'lucide-react';
import { AppIcon, Button, Feedback, Modal } from './ui';
import { desktop } from '../services/desktop';
import { useStore } from '../hooks/store-context';
import { bytes, message } from '../lib/format';
import type { AppRecord, BuildConfig, Prepared } from '../types';

const defaultConfig: BuildConfig = { system: 'cargo', project: '.', target: null, output: null, entry: null, resources: [] };

export function InstallDialog({ app, initialPrepared, onClose }: { app?: AppRecord; initialPrepared?: Prepared; onClose: () => void }) {
  const { busy, progress, run, notify } = useStore();
  const [url, setUrl] = useState(app ? `https://github.com/${app.repository.owner}/${app.repository.name}` : '');
  const [record, setRecord] = useState(app);
  const [prepared, setPrepared] = useState<Prepared | null>(initialPrepared ?? null);
  const [entry, setEntry] = useState(initialPrepared?.entries[0] ?? '');
  const [localError, setLocalError] = useState<string | null>(null);
  const [configure, setConfigure] = useState(false);
  const [config, setConfig] = useState<BuildConfig>(app?.buildConfig ?? defaultConfig);
  const started = useRef(false);
  const install = useCallback(async (configuration: BuildConfig | null = null) => {
    setLocalError(null);
    await run(async () => {
      try {
        const current = record ?? await desktop.add(url, 'main');
        setRecord(current);
        const plan = await desktop.buildSource(current.id, configuration);
        setPrepared(plan); setEntry(plan.entries[0] ?? '');
        if (plan.entries.length === 1) {
          await desktop.commit(plan.ticket, plan.entries[0]);
          notify('Installed and ready to open.'); onClose();
        }
      } catch (error) {
        const detail = message(error); setLocalError(detail);
        if (detail.includes('Build configuration needed:')) setConfigure(true);
      }
    });
  }, [record, url, run, notify, onClose]);
  // Update and repair start from their existing one-click library action.
  useEffect(() => {
    if (app && !initialPrepared && !started.current) { started.current = true; void install(); }
  }, [app, initialPrepared, install]);
  const close = () => {
    if (prepared) notify('Prepared application saved. Resume installation from Storage.');
    onClose();
  };
  const field = (key: 'project' | 'target' | 'output' | 'entry', value: string) => setConfig(old => ({ ...old, [key]: key === 'project' ? value : value || null }));
  return <Modal open onOpenChange={open => { if (!open) close(); }} title={prepared ? 'Choose the application' : app ? `Update ${app.name}` : 'Add an app'} description={prepared ? 'Choose the main application once. Relay remembers it for future updates.' : 'Compile and install the latest source from main.'} busy={busy}>
    {!prepared && <form onSubmit={event => { event.preventDefault(); void install(configure ? config : null); }} className="repository-form">
      <label htmlFor="repository-url">GitHub repository</label><div className="repository-input"><Github size={18} /><input id="repository-url" type="url" required autoFocus={!app} placeholder="https://github.com/you/your-app" value={url} disabled={!!app || busy} onChange={event => { setUrl(event.target.value); setRecord(undefined); setLocalError(null); setConfigure(false); setConfig(defaultConfig); }} /></div>
      {record && <div className="repository-summary"><AppIcon name={record.name} /><div><strong>{record.name}</strong><p>{record.description}</p></div></div>}
      {localError && <><Feedback error>{localError}</Feedback>{!configure && <Button type="button" variant="ghost" disabled={busy} onClick={() => setConfigure(true)}>Change build configuration</Button>}</>}
      {configure && <fieldset className="source-configuration" disabled={busy}><legend>Build configuration</legend><p className="help-text">Choose the project details once. Relay saves them for future updates. Paths are relative to the project folder.</p>
        <label htmlFor="build-system">Build system</label><select id="build-system" className="wide-select" value={config.system} onChange={e => setConfig(old => ({ ...old, system: e.target.value as BuildConfig['system'] }))}>{Object.entries({ tauri: 'Tauri', electron: 'Electron', cargo: 'Rust / Cargo', cmake: 'CMake', maven: 'Java / Maven', gradle: 'Java / Gradle', python: 'Python / PyInstaller', go: 'Go', dotnet: '.NET' }).map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select>
        <label htmlFor="build-project">Project folder</label><input id="build-project" required value={config.project} onChange={e => field('project', e.target.value)} placeholder="." />
        <label htmlFor="build-target">Target or source entry, if needed</label><input id="build-target" value={config.target ?? ''} onChange={e => field('target', e.target.value)} placeholder="For example: app.py, cmd/app, Desktop.csproj" />
        <label htmlFor="build-output">Built application folder, if needed</label><input id="build-output" value={config.output ?? ''} onChange={e => field('output', e.target.value)} placeholder="Detect automatically" />
        <label htmlFor="build-entry">Launch file within the built application, if needed</label><input id="build-entry" value={config.entry ?? ''} onChange={e => field('entry', e.target.value)} placeholder="Choose automatically" />
        <label htmlFor="build-resources">Additional resource paths, one per line</label><textarea id="build-resources" value={config.resources.join('\n')} onChange={e => setConfig(old => ({ ...old, resources: e.target.value.split('\n').filter(Boolean) }))} placeholder="assets" />
      </fieldset>}
      <p className="help-text">Install runs this repository's build code on your computer. Only install repositories you trust. The required compiler or SDK must be installed.</p>
      <div className="dialog-footer"><Button type="button" variant="ghost" disabled={busy} onClick={close}>Cancel</Button><Button type="submit" variant="primary" disabled={busy || !url || (configure && !config.project)}><Download size={16} />{configure ? 'Save and retry' : localError ? 'Retry installation' : app?.activeVersion ? 'Update' : 'Install'}</Button></div>
    </form>}
    {prepared && <div className="entry-choice"><CheckCircle2 size={26} className="success-text" /><p>The application is built and ready to install.</p><label htmlFor="launch-file">Application to open</label><select id="launch-file" className="wide-select" value={entry} disabled={busy} onChange={e => setEntry(e.target.value)}>{prepared.entries.map(path => <option key={path}>{path}</option>)}</select>{localError && <Feedback error>{localError}</Feedback>}<div className="dialog-footer"><Button disabled={busy} onClick={() => void run(async () => { await desktop.perform({ type: 'discard', ticket: prepared.ticket }); onClose(); })}>Discard build</Button><Button variant="primary" disabled={busy || !entry} onClick={() => void run(async () => { await desktop.commit(prepared.ticket, entry); notify('Installed and ready to open.'); onClose(); })}>Finish installation<ArrowRight size={15} /></Button></div></div>}
    {!prepared && !localError && <div className="install-assurance"><ShieldCheck size={17} /><span>Build files and dependency caches are removed automatically.<br />Your settings and previous version are kept.</span></div>}
    {busy && <div className="dialog-operation" role="status" aria-live="polite"><strong>{progress?.message ?? 'Preparing your application...'}</strong>{progress?.total ? <><progress max={progress.total} value={progress.completed} aria-label="Source download" /><span>{bytes(progress.completed)} of {bytes(progress.total)}</span></> : <progress aria-label="Installation progress" />}<Button variant="ghost" onClick={() => void desktop.cancel()}>Cancel operation</Button></div>}
  </Modal>;
}
