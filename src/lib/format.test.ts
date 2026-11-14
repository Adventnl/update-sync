import { describe, expect, it } from 'vitest';
import { bytes, date, hasUpdate } from './format';
import type { AppRecord } from '../types';

describe('display models', () => {
  it('handles unknown storage values and invalid dates', () => {
    expect(bytes(0)).toBe('0 B'); expect(bytes(NaN)).toBe('0 B'); expect(bytes(1024)).toBe('1 KB'); expect(date('invalid')).toBe('Unknown date');
  });
  it('does not call an uninstalled app an update', () => {
    const app = { activeVersion: null, versions: [], available: { id: 'release-1' } } as unknown as AppRecord;
    expect(hasUpdate(app)).toBe(false);
    const installed = { ...app, activeVersion: '1', versions: [{ id: '1', source: 'release-1' }] } as unknown as AppRecord;
    expect(hasUpdate(installed)).toBe(false);
    expect(hasUpdate({ ...installed, available: { id: 'release-2' } } as AppRecord)).toBe(true);
  });
});
