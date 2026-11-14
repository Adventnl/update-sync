import { createContext, useContext } from 'react';
import type { Action, Progress, Snapshot } from '../types';

export interface Notice { id: number; text: string; error: boolean }
export interface StoreContextValue {
  snapshot: Snapshot | null; loading: boolean; error: string | null;
  busy: boolean; progress: Progress | null; notices: Notice[];
  refresh: () => Promise<void>; dismiss: (id: number) => void;
  notify: (text: string, error?: boolean) => void;
  run: <T>(task: () => Promise<T>, success?: string) => Promise<T | undefined>;
  act: (action: Action, success?: string) => Promise<void>;
}
export const StoreContext = createContext<StoreContextValue | null>(null);
export function useStore() {
  const context = useContext(StoreContext);
  if (!context) throw new Error('StoreProvider is missing.');
  return context;
}
