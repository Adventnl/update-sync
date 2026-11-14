import * as Dialog from '@radix-ui/react-dialog';
import { X, Package, Check, AlertCircle } from 'lucide-react';
import type { ButtonHTMLAttributes, CSSProperties, ReactNode } from 'react';
import { appHue } from '../lib/format';

export function Button({ children, variant = 'secondary', className = '', ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'primary' | 'secondary' | 'ghost' | 'danger' }) {
  return <button {...props} className={`button ${variant} ${className}`}>{children}</button>;
}
export function AppIcon({ name, size = 'normal' }: { name: string; size?: 'normal' | 'large' | 'small' }) {
  return <div className={`app-icon ${size}`} aria-hidden="true" style={{ '--app-hue': appHue(name) } as CSSProperties}>{name.slice(0, 1).toUpperCase()}</div>;
}
export function Modal({ open, onOpenChange, title, description, children, className = '', busy = false }: { open: boolean; onOpenChange: (open: boolean) => void; title: string; description?: string; children: ReactNode; className?: string; busy?: boolean }) {
  return <Dialog.Root open={open} onOpenChange={value => { if (!busy) onOpenChange(value); }}><Dialog.Portal><Dialog.Overlay className="overlay" /><Dialog.Content className={`dialog ${className}`} onPointerDownOutside={e => e.preventDefault()}>
    <div className="dialog-heading"><Dialog.Title>{title}</Dialog.Title><Dialog.Close className="icon-button" aria-label="Close dialog" disabled={busy}><X size={18} /></Dialog.Close></div>
    <Dialog.Description className={description ? 'muted dialog-description' : 'sr-only'}>{description ?? title}</Dialog.Description>{children}
  </Dialog.Content></Dialog.Portal></Dialog.Root>;
}
export function EmptyState({ title, children, action, icon = <Package size={30} strokeWidth={1.5} /> }: { title: string; children: ReactNode; action?: ReactNode; icon?: ReactNode }) {
  return <div className="empty-state"><div className="empty-icon">{icon}</div><h2>{title}</h2><p>{children}</p>{action}</div>;
}
export function Feedback({ children, error = false }: { children: ReactNode; error?: boolean }) {
  const Icon = error ? AlertCircle : Check;
  return <div className={`feedback ${error ? 'error' : ''}`} role={error ? 'alert' : 'status'}><Icon size={17} /><span>{children}</span></div>;
}
