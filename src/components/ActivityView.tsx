import { Check, Clock3, Download, Link2, Trash2, RotateCcw } from 'lucide-react';
import { useStore } from '../hooks/store-context';
import { date } from '../lib/format';
import { EmptyState } from './ui';

function eventIcon(action: string) {
  if (action.includes('Uninstall')) return Trash2;
  if (action.includes('restored')) return RotateCcw;
  if (action.includes('Tracking')) return Link2;
  if (action === 'Installed' || action === 'Updated') return Download;
  return Check;
}
export function ActivityView({ query }: { query: string }) {
  const { snapshot } = useStore();
  const events = snapshot?.activity.filter(e => `${e.appName} ${e.action} ${e.detail}`.toLowerCase().includes(query.toLowerCase())) ?? [];
  return <section className="activity-view"><div className="page-heading"><div><h1>A clear history.</h1><p>Every install, update, and change, right here.</p></div></div>{events.length ? <div className="activity-list">{events.map(event => { const Icon = eventIcon(event.action); return <div className="activity-row" key={event.id}><div className="activity-icon"><Icon size={17} /></div><div><strong>{event.appName}<span>{event.action}</span></strong><p>{event.detail}</p></div><time dateTime={event.at}>{date(event.at)}</time></div>; })}</div> : <EmptyState title={query ? 'No matching activity' : 'A fresh start'} icon={<Clock3 size={28} />}>{query ? 'Try another app name or action.' : "Your app history will appear here after you add your first repository."}</EmptyState>}</section>;
}
