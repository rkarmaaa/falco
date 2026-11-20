// Stato condiviso della UI, idratato da un unico state.snapshot all'avvio.
import { reactive, readonly } from 'vue';
import {
  getTransport,
  rpc,
  type FaloConfig,
  type HostState,
  type PairedClient,
  type PendingPairing,
  type Recommendation,
  type SessionInfo,
  type SessionStats,
  type StateSnapshot,
  type Status,
} from '@/ipc';

/** `linking`: invito o link di stream, handshake e pairing in corso prima di bussare. */
export type SessionPhase = 'idle' | 'linking' | 'starting' | 'connecting' | 'connected';

interface CoreStore {
  ready: boolean;
  /** Sessione che si sta riaprendo (cambio di qualità): niente avviso di fine. */
  reconnecting: boolean;
  connected: boolean;
  snapshot: StateSnapshot | null;
  config: FaloConfig | null;
  host: HostState;
  session: {
    phase: SessionPhase;
    host: string | null;
    /** Amico da cui si entra, noto da subito (foto sul palco prima che parta Moonlight). */
    friendId: string | null;
    /** La sessione è partita da un invito: il palco mostra anche il passo del collegamento. */
    linked: boolean;
    info: SessionInfo | null;
    stats: SessionStats | null;
  };
  lastSession: { host: string; durationS: number; reason: string; latencyMs: number | null; friendId: string | null } | null;
  pairing: { host: string; pin: string; result: 'pending' | 'ok' | 'failed' } | null;
}

const state = reactive<CoreStore>({
  ready: false,
  reconnecting: false,
  connected: true,
  snapshot: null,
  config: null,
  host: { state: 'stopped' },
  session: { phase: 'idle', host: null, friendId: null, linked: false, info: null, stats: null },
  lastSession: null,
  pairing: null,
});

const idle = (): CoreStore['session'] => ({ phase: 'idle', host: null, friendId: null, linked: false, info: null, stats: null });

async function refresh(): Promise<void> {
  const [snapshot, { config }] = await Promise.all([
    rpc<StateSnapshot>('state.snapshot'),
    rpc<{ config: FaloConfig }>('config.get'),
  ]);
  state.snapshot = snapshot;
  state.config = config;
  state.host = snapshot.host;
  if (!snapshot.streaming) state.session = idle();
  else if (snapshot.session) {
    // Finestra riaperta durante una sessione: si riprende da dove era
    state.session.info = snapshot.session;
    state.session.host = snapshot.session.name;
    if (state.session.phase === 'idle') state.session.phase = 'connected';
  }
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
      case 'session.started':
        state.session.info = p as unknown as SessionInfo;
        state.session.host = String(p.name ?? p.host);
        if (['idle', 'linking', 'starting'].includes(state.session.phase)) state.session.phase = 'connecting';
        break;
      case 'session.stats':
        if (p.phase === 'connecting' || p.phase === 'connected') state.session.phase = p.phase;
        if (p.phase === 'stats') {
          state.session.phase = 'connected';
          state.session.stats = p as unknown as SessionStats;
        }
        break;
      case 'session.ended':
        state.lastSession = {
          host: String(state.session.host ?? p.host),
          durationS: Number(p.durationS),
          reason: String(p.reason ?? ''),
          latencyMs: typeof p.latencyMs === 'number' ? p.latencyMs : null,
          friendId: state.session.info?.friendId ?? null,
        };
        state.session = idle();
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
  state.session = { ...idle(), phase: 'starting', host: address };
  try {
    await rpc('session.join', { ...target(address), app });
  } catch (e) {
    state.session = idle();
    throw e;
  }
}

/** Palco subito con la foto dell'host mentre un invito collega i due PC (handshake e pairing). */
export function beginLink(friendId: string, name: string): void {
  state.session = { ...idle(), phase: 'linking', host: name, friendId, linked: true };
}

/** Collegamento fallito o annullato: si torna alla lista. */
export function endLink(): void {
  if (state.session.phase === 'linking') state.session = idle();
}

export async function joinFriend(friendId: string, name: string): Promise<void> {
  const linked = state.session.linked && state.session.friendId === friendId;
  state.session = { ...idle(), phase: 'starting', host: name, friendId, linked };
  try {
    await rpc('session.join', { friendId });
  } catch (e) {
    state.session = idle();
    throw e;
  }
}

export async function leaveSession(): Promise<void> {
  await rpc('session.leave');
}

/** Riapre la sessione con le impostazioni nuove (es. qualità), senza tornare alla lista amici. */
export async function reconnectSession(): Promise<void> {
  const friendId = state.session.info?.friendId;
  const name = state.session.host ?? '';
  if (!friendId) return;
  state.reconnecting = true;
  try {
    await leaveSession();
    for (let i = 0; i < 50 && state.session.phase !== 'idle'; i++) await new Promise((r) => setTimeout(r, 100));
    await joinFriend(friendId, name);
  } finally {
    state.reconnecting = false;
  }
}

export function clearLastSession(): void {
  state.lastSession = null;
}

export const hostApi = {
  recommend: () => rpc<Recommendation>('host.recommend'),
  applyInputs: () => rpc('host.applyInputs'),
  pairings: () => rpc<PendingPairing[]>('host.pairings'),
  clients: () => rpc<PairedClient[]>('host.clients'),
  approve: (id: string, pin: string, name: string) => rpc('host.approvePairing', { id, pin, name }),
  unpair: (uuid: string) => rpc('host.unpair', { uuid }),
  setEnabled: (uuid: string, enabled: boolean) => rpc('host.setClientEnabled', { uuid, enabled }),
  endSession: () => rpc('host.endSession'),
};

export const core = readonly(state);
