/**
 * "Install driver" for a Windows USBTMC instrument that has no usable driver
 * (`POST /usbtmc-devices/driver`, ai_context/migration/IDENTITY.md): the server gives it
 * Microsoft's generic WinUSB driver after Windows asks the user for administrator approval.
 * The request waits for that answer, so the button stays busy meanwhile. Admin only: a remote
 * browser does not get the button (the prompt would appear on the bench computer).
 */

import { useState } from 'react';
import { apiError } from './detect';
import { useIsRemote } from '../remote/RemoteProvider';

/** Windows re-enumerates the instrument after the driver change; lists are re-read after this. */
export const REENUMERATE_MS = 2000;

export async function installDriver(apiBase: string, device: string): Promise<void> {
  const resp = await fetch(`${apiBase}/usbtmc-devices/driver`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ device }),
  });
  if (!resp.ok) throw new Error(await apiError(resp));
}

interface Props {
  apiBase: string;
  device: string;
  /** The driver is installed; called once Windows had time to re-enumerate the instrument. */
  onInstalled: () => void;
}

export function InstallDriverButton({ apiBase, device, onInstalled }: Props) {
  const [state, setState] = useState<'idle' | 'busy' | 'done'>('idle');
  const [error, setError] = useState<string | null>(null);
  const remote = useIsRemote();

  async function install() {
    setState('busy');
    setError(null);
    try {
      await installDriver(apiBase, device);
      setState('done');
      setTimeout(onInstalled, REENUMERATE_MS);
    } catch (e: any) {
      console.error(`Driver installation for ${device} failed:`, e);
      setError(e?.message || String(e));
      setState('idle');
    }
  }

  if (remote) return null;
  return (
    <div className="install-driver" data-testid="install-driver">
      <button
        type="button" className="detect__btn" onClick={install} disabled={state !== 'idle'}
        title="Give this instrument Microsoft's generic WinUSB driver (Windows asks for administrator approval once)"
      >
        {state === 'busy' ? 'Waiting for Windows approval…' : state === 'done' ? '✓ Driver installed' : 'Install driver'}
      </button>
      {error && <div className="instrument-config__inline-error">{error}</div>}
    </div>
  );
}
