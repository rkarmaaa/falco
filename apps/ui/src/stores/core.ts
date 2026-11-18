// Stato condiviso della UI, idratato da un unico state.snapshot all'avvio.
import { reactive, readonly } from 'vue';
import {
  getTransport,
  rpc,
  type FaloConfig,
  type HostState,
  type PairedClient,
  type PendingPairing,
  type StateSnapshot,
  type Status,
} from '@/ipc';

export type SessionPhase = 'idle' | 'starting' | 'connecting' | 'connected';

interface CoreStore {
  ready: boolean;
  connected: boolean;
  snapshot: StateSnapshot | null;
  config: FaloConfig | null;
  host: HostState;
  session: { phase: SessionPhase; host: string | null };
  lastSession: { host: string; durationS: number } | null;
  pairing: { host: string; pin: string; result: 'pending' | 'ok' | 'failed' } | null;
}

const state = reactive<CoreStore>({
  ready: false,
  connected: true,
  snapshot: null,
  config: null,
  host: { state: 'stopped' },
  session: { phase: 'idle', host: null },
  lastSession: null,
  pairing: null,
});

async function refresh(): Promise<void> {
  const [snapshot, { config }] = await Promise.all([
    rpc<StateSnapshot>('state.snapshot'),
    rpc<{ config: FaloConfig }>('config.get'),
  ]);
  state.snapshot = snapshot;
  state.config = config;
  state.host = snapshot.host;
  if (!snapshot.streaming) state.session = { phase: 'idle', host: null };
  state.connected = true;
}

export async function initCore(): Promise<void> {
  const t = await getTransport();
  t.onConnection((ok) => {
    state.connected = ok;
    if (ok) void refresh();
  });
  t.onEvent((e) => {
    const p = e.params as Record<string, unknown>;
    switch (e.method) {
      case 'presence.changed': {
        const self = p.self as Partial<StateSnapshot> | undefined;
        if (self && state.snapshot) Object.assign(state.snapshot, self);
        break;
      }
      case 'config.changed':
        void refresh();
        break;
      case 'host.state':
        state.host = p as unknown as HostState;
        break;
      case 'pair.result':
        if (state.pairing) state.pairing.result = p.ok ? 'ok' : 'failed';
        break;
      case 'session.stats':
        if (p.phase === 'connecting' || p.phase === 'connected') state.session.phase = p.phase;
        break;
      case 'session.ended':
        state.session = { phase: 'idle', host: null };
        state.lastSession = { host: String(p.host), durationS: Number(p.durationS) };
        break;
    }
  });
  try {
    await refresh();
  } catch {
    state.connected = false;
  }
  state.ready = true;
}

export async function setStatus(status: Status): Promise<void> {
  await rpc('profile.update', { status });
}

export async function setHostAvailable(available: boolean): Promise<void> {
  if (available) state.host = { state: 'starting' };
  await rpc('host.setAvailable', { available });
}

export async function setConfig(key: string, value: unknown): Promise<void> {
  await rpc('config.set', { key, value });
}

/** Separa "host:porta"; senza porta vale quella standard. */
function target(address: string): { host: string; port?: number } {
  const m = /^(.+):(\d+)$/.exec(address.trim());
  return m ? { host: m[1]!, port: Number(m[2]) } : { host: address.trim() };
}

export async function pairWith(address: string): Promise<void> {
  const r = await rpc<{ pin: string; host: string }>('session.pair', target(address));
  state.pairing = { host: r.host, pin: r.pin, result: 'pending' };
}

export async function join(address: string, app = 'Desktop'): Promise<void> {
  state.session = { phase: 'starting', host: address };
  try {
    await rpc('session.join', { ...target(address), app });
  } catch (e) {
    state.session = { phase: 'idle', host: null };
    throw e;
  }
}

export const hostApi = {
  pairings: () => rpc<PendingPairing[]>('host.pairings'),
  clients: () => rpc<PairedClient[]>('host.clients'),
  approve: (id: string, pin: string, name: string) => rpc('host.approvePairing', { id, pin, name }),
  unpair: (uuid: string) => rpc('host.unpair', { uuid }),
  setEnabled: (uuid: string, enabled: boolean) => rpc('host.setClientEnabled', { uuid, enabled }),
  endSession: () => rpc('host.endSession'),
};

export const core = readonly(state);
