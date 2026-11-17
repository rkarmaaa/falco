// Core simulato per il browser: stesso contratto JSON-RPC, dati finti.
import type { Transport } from './index';
import { NOT_IMPLEMENTED, type CoreEvent, type StateSnapshot } from './types';

export function createMockTransport(): Transport {
  const listeners: ((e: CoreEvent) => void)[] = [];
  const emit = (method: string, params: unknown) => listeners.forEach((cb) => cb({ method, params }));

  const state: StateSnapshot = {
    version: '0.1.0',
    milestone: 'M0',
    status: 'online',
    hostAvailable: false,
    streaming: false,
    network: { provider: 'tailscale', tailscale: 'running', selfIps: ['100.101.102.103'], exitNodeActive: false },
    friendsCount: 0,
  };
  const config = {
    general: { language: 'it', updates: 'auto', theme: 'system', start_with_system: true, start_minimized: true, close_to_tray: true },
    profile: { status: 'online' },
    stream: {},
    host: { max_guests: 3, encoder: 'auto', default_role: 'spettatore' },
    network: { provider: 'tailscale', presence_port: 47900, fec_percent: 20 },
    performance: {},
    advanced: {},
  } as Record<string, Record<string, unknown>>;

  const handlers: Record<string, (p: any) => unknown> = {
    'state.snapshot': () => structuredClone(state),
    'config.get': () => ({ config: structuredClone(config) }),
    'config.set': ({ key, value }: { key: string; value: unknown }) => {
      const [section, name] = key.split('.') as [string, string];
      (config[section] ??= {})[name] = value;
      emit('config.changed', { key, value });
      return { applyAt: 'now' };
    },
    'profile.update': ({ status }: { status: StateSnapshot['status'] }) => {
      state.status = status;
      emit('presence.changed', { self: { status } });
      return null;
    },
    'host.setAvailable': ({ available }: { available: boolean }) => {
      state.hostAvailable = available;
      emit('presence.changed', { self: { hostAvailable: available } });
      return { available };
    },
    'friends.list': () => [],
    'net.runDoctor': () => ({
      checks: [
        { id: 'tailscaleInstalled', level: 'ok', value: null, action: null },
        { id: 'tailscaleRunning', level: 'ok', value: '100.101.102.103', action: null },
        { id: 'peersRelay', level: 'warn', value: '1', action: 'enableUpnp' },
        ...['udp', 'sunshinePorts', 'bandwidth', 'jitter', 'localSetup'].map((id) => ({
          id,
          level: 'pending',
          value: null,
          action: null,
        })),
      ],
    }),
  };

  return {
    async call<T>(method: string, params?: unknown): Promise<T> {
      await new Promise((r) => setTimeout(r, 60));
      const h = handlers[method];
      if (!h) throw { code: NOT_IMPLEMENTED, message: 'non ancora disponibile', data: { milestone: 'M3' } };
      return h(params ?? {}) as T;
    },
    onEvent: (cb) => void listeners.push(cb),
    onConnection: () => {},
    connected: async () => true,
  };
}
