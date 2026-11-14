import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from '../App';
import type { AppRecord, Channel, Inspection, Snapshot, StorageItem } from '../types';
import { desktop } from '../services/desktop';

vi.mock('../services/desktop', () => ({ desktop: { snapshot: vi.fn(), listen: vi.fn().mockResolvedValue(() => {}), inspect: vi.fn(), add: vi.fn(), prepare: vi.fn(), commit: vi.fn(), perform: vi.fn(), storage: vi.fn(), prepared: vi.fn(), settings: vi.fn(), token: vi.fn(), folder: vi.fn(), cancel: vi.fn(), buildPlan: vi.fn(), build: vi.fn(), workflows: vi.fn(), scan: vi.fn(), removeCache: vi.fn(), cleanup: vi.fn() } }));
const snapshot: Snapshot = { apps: [], activity: [], root: 'test-library', platform: 'windows', architecture: 'x86_64', auth: 'Public access', settings: { appearance: 'system', useGithubCli: true, checkOnOpen: false } };
const inspection: Inspection = { repository: { owner: 'test', name: 'Example' }, repositoryId: 1, description: 'Test fixture application', defaultBranch: 'main', headCommit: 'a'.repeat(40), stacks: ['Rust'], packages: [{ id: 'release-1', name: 'Example-windows.zip', version: 'v1', url: 'https://api.github.com/repos/test/Example/releases/assets/1', commit: null, digest: null, size: 1024, source: 'release', publishedAt: '', compatible: true }], warnings: [], htmlUrl: 'https://github.com/test/Example' };
beforeEach(() => {
  vi.clearAllMocks(); vi.mocked(desktop.snapshot).mockResolvedValue(structuredClone(snapshot));
  vi.mocked(desktop.storage).mockResolvedValue([]); vi.mocked(desktop.prepared).mockResolvedValue([]);
});

function record(name: string, id: number, channel: Channel): AppRecord {
  return { id: `gh-${id}`, repositoryId: id, repository: { owner: 'test', name }, name, description: 'Test fixture', stacks: ['Rust'], channel, tracked: true, activeVersion: null, versions: [], available: null, checkedAt: null, checkError: null, desktopShortcut: false, dataFolders: [] };
}

describe('library workflow', () => {
  it('starts with real empty state and opens repository entry', async () => {
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('heading', { name: /Your next app/ });
    await user.click(screen.getByRole('button', { name: 'Add your first app' }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'GitHub repository' })).toHaveFocus();
  });
  it('requires explicit approval when the package has no checksum', async () => {
    const user = userEvent.setup(); vi.mocked(desktop.inspect).mockResolvedValue(inspection); render(<App />);
    await user.click(await screen.findByRole('button', { name: 'Add your first app' }));
    await user.type(screen.getByRole('textbox', { name: 'GitHub repository' }), 'https://github.com/test/Example');
    await user.click(screen.getByRole('button', { name: 'Find app' }));
    const install = await screen.findByRole('button', { name: 'Install app' });
    expect(install).toBeDisabled();
    await user.click(screen.getByRole('checkbox', { name: /without a publisher checksum/ }));
    expect(install).toBeEnabled();
    expect(desktop.prepare).not.toHaveBeenCalled();
  });
  it('keeps failed inspection recoverable and shows the reason', async () => {
    const user = userEvent.setup(); vi.mocked(desktop.inspect).mockRejectedValue(new Error('Repository not found.')); render(<App />);
    await user.click(await screen.findByRole('button', { name: 'Add your first app' }));
    await user.type(screen.getByRole('textbox', { name: 'GitHub repository' }), 'https://github.com/test/missing');
    await user.click(screen.getByRole('button', { name: 'Find app' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Repository not found.');
    expect(screen.getByRole('button', { name: 'Find app' })).toBeEnabled();
  });
  it('navigates to real storage and respects system appearance', async () => {
    const user = userEvent.setup(); render(<App />);
    await screen.findByRole('heading', { name: 'Your library' });
    await user.click(screen.getByRole('button', { name: 'Storage' }));
    expect(await screen.findByText('Your managed storage is tidy.')).toBeInTheDocument();
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('system'));
  });
  it('opens contextual actions through an accessibility click without pointer events', async () => {
    vi.mocked(desktop.snapshot).mockResolvedValue({ ...structuredClone(snapshot), apps: [{ id: 'gh-1', repositoryId: 1, repository: { owner: 'test', name: 'Example' }, name: 'Example', description: 'Test fixture', stacks: ['Rust'], channel: 'stable', tracked: true, activeVersion: null, versions: [], available: null, checkedAt: null, checkError: null, desktopShortcut: false, dataFolders: [] }] });
    render(<App />);
    const trigger = await screen.findByRole('button', { name: 'Actions for Example' });
    fireEvent.click(trigger, { detail: 1 });
    expect(await screen.findByRole('menuitem', { name: 'App details' })).toBeInTheDocument();
  });
  it('invalidates the retained app and build trust when repository or channel changes', async () => {
    const user = userEvent.setup();
    const second: Inspection = { ...inspection, repositoryId: 2, repository: { owner: 'test', name: 'Second' } };
    vi.mocked(desktop.inspect).mockResolvedValueOnce(inspection).mockResolvedValueOnce(second).mockResolvedValueOnce(second);
    vi.mocked(desktop.add).mockImplementation(async (url, channel) => url.endsWith('/Second') ? record('Second', 2, channel) : record('Example', 1, channel));
    vi.mocked(desktop.buildPlan).mockResolvedValue({ stack: 'Rust', steps: ['cargo build --release'], warning: 'Trust the repository.' });
    render(<App />);
    await user.click(await screen.findByRole('button', { name: 'Add your first app' }));
    const field = screen.getByRole('textbox', { name: 'GitHub repository' });
    await user.type(field, 'https://github.com/test/Example'); await user.click(screen.getByRole('button', { name: 'Find app' }));
    await user.click(await screen.findByRole('button', { name: 'Review local build…' }));
    await user.click(await screen.findByRole('checkbox', { name: /approve running its build code/ }));
    await user.clear(field); await user.type(field, 'https://github.com/test/Second');
    await user.click(screen.getByRole('button', { name: 'Find app' }));
    await user.click(await screen.findByRole('button', { name: 'Review local build…' }));
    expect(await screen.findByRole('checkbox', { name: /approve running its build code/ })).not.toBeChecked();
    expect(desktop.buildPlan).toHaveBeenLastCalledWith('gh-2');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Channel' }), 'main');
    await user.click(screen.getByRole('button', { name: 'Find app' }));
    await user.click(await screen.findByRole('button', { name: 'Review local build…' }));
    expect(desktop.add).toHaveBeenLastCalledWith('https://github.com/test/Second', 'main');
    expect(await screen.findByRole('checkbox', { name: /approve running its build code/ })).not.toBeChecked();
  });
  it('keeps storage failures separate and never reports failed measurements as tidy', async () => {
    const user = userEvent.setup();
    vi.mocked(desktop.storage).mockRejectedValue(new Error('Disk unavailable.'));
    vi.mocked(desktop.prepared).mockRejectedValue(new Error('Download receipt unavailable.'));
    render(<App />); await screen.findByRole('heading', { name: 'Your library' });
    await user.click(screen.getByRole('button', { name: 'Storage' }));
    await screen.findByText(/Managed storage could not be measured/);
    expect(screen.queryByText('Your managed storage is tidy.')).not.toBeInTheDocument();
    expect(screen.queryByText('0 B available')).not.toBeInTheDocument();
    vi.mocked(desktop.storage).mockResolvedValue([]);
    await user.click(screen.getByRole('button', { name: 'Try again' }));
    await screen.findByText('Your managed storage is tidy.');
    expect(screen.getByText(/Prepared downloads are unavailable/)).toBeInTheDocument();
  });
  it('preserves the successful scan root and reconciles partially deleted caches', async () => {
    const user = userEvent.setup();
    const caches: StorageItem[] = ['one', 'two'].map((name, i) => ({ id: `cache-${name}`, name: `Cache ${name}`, path: `C:/Projects/A/${name}/target`, size: 1024 + i, kind: 'cache', appId: null, detail: 'Recognized test cache' }));
    vi.mocked(desktop.folder).mockResolvedValueOnce('C:/Projects/A').mockResolvedValueOnce('C:/Projects/B');
    vi.mocked(desktop.scan).mockResolvedValueOnce(caches).mockRejectedValueOnce(new Error('Scan denied.'));
    vi.mocked(desktop.removeCache).mockResolvedValueOnce(1024).mockRejectedValueOnce(new Error('File locked.'));
    render(<App />); await screen.findByRole('heading', { name: 'Your library' }); await user.click(screen.getByRole('button', { name: 'Storage' }));
    await user.click(screen.getByRole('button', { name: 'Choose folder…' })); await screen.findByText('Cache one');
    await user.click(screen.getByRole('button', { name: 'Choose folder…' })); await screen.findByText(/The cache scan did not finish/);
    expect(screen.getByText('Scanned C:/Projects/A')).toBeInTheDocument(); expect(screen.queryByText('Scanned C:/Projects/B')).not.toBeInTheDocument();
    await user.click(screen.getByRole('checkbox', { name: /Cache one/ })); await user.click(screen.getByRole('checkbox', { name: /Cache two/ }));
    await user.click(screen.getByRole('button', { name: 'Clean up' })); await user.click(screen.getByRole('button', { name: 'Remove selected files' }));
    await waitFor(() => expect(screen.queryByText('Cache one')).not.toBeInTheDocument());
    expect(desktop.removeCache).toHaveBeenNthCalledWith(1, 'C:/Projects/A', caches[0].path);
    expect(desktop.removeCache).toHaveBeenNthCalledWith(2, 'C:/Projects/A', caches[1].path);
    expect(screen.getAllByText('Cache two').length).toBeGreaterThan(0);
  });
});
