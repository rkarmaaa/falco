// Stato condiviso della UI, idratato da un unico state.snapshot all'avvio.
import { reactive, readonly } from 'vue';
import {
  getTransport,
  rpc,
  type FaloConfig,
  type StateSnapshot,
  type Status,
} from '@/ipc';

interface CoreStore {
  ready: boolean;
  connected: boolean;
  snapshot: StateSnapshot | null;
  config: FaloConfig | null;
}

const state = reactive<CoreStore>({ ready: false, connected: true, snapshot: null, config: null });

async function refresh(): Promise<void> {
  const [snapshot, { config }] = await Promise.all([
    rpc<StateSnapshot>('state.snapshot'),
    rpc<{ config: FaloConfig }>('config.get'),
  ]);
  state.snapshot = snapshot;
  state.config = config;
  state.connected = true;
}

export async function initCore(): Promise<void> {
  const t = await getTransport();
  t.onConnection((ok) => {
    state.connected = ok;
    if (ok) void refresh();
  });
  t.onEvent((e) => {
    if (e.method === 'presence.changed' && state.snapshot) {
      const self = (e.params as { self?: Partial<StateSnapshot> }).self;
      if (self) Object.assign(state.snapshot, self);
    }
    if (e.method === 'config.changed') void refresh();
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
  await rpc('host.setAvailable', { available });
}

export async function setConfig(key: string, value: unknown): Promise<void> {
  await rpc('config.set', { key, value });
}

export const core = readonly(state);
