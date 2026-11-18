// Core simulato per il browser: stesso contratto JSON-RPC, dati finti.
import type { Transport } from './index';
import { NOT_IMPLEMENTED, type CoreEvent, type StateSnapshot } from './types';

export function createMockTransport(): Transport {
  const listeners: ((e: CoreEvent) => void)[] = [];
  const emit = (method: string, params: unknown) => listeners.forEach((cb) => cb({ method, params }));

  const state: StateSnapshot = {
    version: '1.0.0-rc.1',
    milestone: 'M6',
    host: { state: 'stopped' },
    status: 'online',
    hostAvailable: false,
    streaming: false,
    network: { provider: 'tailscale', tailscale: 'running', selfIps: ['100.101.102.103'], exitNodeActive: false },
    friendsCount: 2,
    hasProfile: true,
    oauthConfigured: false,
  };
  const profile = { exists: true, id: 'ME', tag: '#K7QM-2XPA', accent: '#FF6A2B', name: 'Marco', avatarUrl: null as string | null };
  const friends = [
    { id: 'L', tag: '#AXBR-7FQZ', name: 'Luca', accent: '#3D8BDC', avatarUrl: null, state: 'playing', canJoin: true, slotsFree: 2, favorite: true, lastSeen: null, role: 'giocatore', devices: [{ tsNodeId: 'n1', tsIp: '100.64.0.2', hostname: 'luca-pc', os: 'windows', canHost: true }] },
    { id: 'B', tag: '#Q2LM-88TA', name: 'Bea', accent: '#C04BD6', avatarUrl: null, state: 'online', canJoin: false, slotsFree: null, favorite: false, lastSeen: null, role: 'copilota', devices: [] },
    { id: 'G', tag: '#ZZ4K-1PRT', name: 'Gio', accent: '#3DDC84', avatarUrl: null, state: 'offline', canJoin: false, slotsFree: null, favorite: false, lastSeen: Math.floor(Date.now() / 1000) - 7200, role: 'spettatore', devices: [] },
  ];
  const invites: { tokenHash: string; role: string; createdAt: number; expiresAt: number; usesLeft: number; needsApproval: boolean; active: boolean }[] = [];
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
    'config.schema': () => [
      { key: 'stream.fps', type: 'autoInt', options: [30, 60, 90, 120], applyAt: 'nextSession' },
      { key: 'stream.vsync', type: 'bool', applyAt: 'nextSession' },
      { key: 'network.provider', type: 'enum', options: ['tailscale', 'lan', 'direct', 'headscale'], applyAt: 'now' },
      { key: 'network.fec_percent', type: 'int', min: 0, max: 100, applyAt: 'hostRestart' },
      { key: 'advanced.log_level', type: 'enum', options: ['error', 'warn', 'info', 'debug'], applyAt: 'now' },
    ],
    'config.export': () => ({ text: 'schema = 1' }),
    'config.import': () => null,
    'update.check': () => ({ available: { version: '1.0.0', notes: 'Prima versione stabile.' }, current: '1.0.0-rc.1', enabled: true }),
    'update.install': () => null,
    'tailscale.install': () => null,
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
      state.host = { state: available ? 'starting' : 'stopped' };
      emit('host.state', state.host);
      emit('presence.changed', { self: { hostAvailable: available } });
      if (available)
        setTimeout(() => {
          state.host = { state: 'ready' };
          emit('host.state', state.host);
        }, 1500);
      return { available };
    },
    'host.pairings': () => (state.host.state === 'ready' ? [{ id: 'p1', name: 'roth', address: '100.64.0.2' }] : []),
    'host.clients': () => [{ uuid: 'c1', name: 'MacBook di Luca', enabled: true }],
    'host.approvePairing': () => null,
    'host.unpair': () => null,
    'host.setClientEnabled': () => null,
    'host.endSession': () => null,
    'session.pair': ({ host, port }: { host: string; port?: number }) => {
      setTimeout(() => emit('pair.result', { host, ok: true }), 2500);
      return { pin: '4821', host: port ? `${host}:${port}` : host };
    },
    'session.join': ({ host }: { host: string }) => {
      setTimeout(() => emit('session.stats', { phase: 'connecting' }), 400);
      setTimeout(() => emit('session.stats', { phase: 'connected' }), 1200);
      setTimeout(() => emit('session.ended', { host, durationS: 5, reason: '0' }), 5000);
      return null;
    },
    'friends.list': () => structuredClone(friends),
    'friends.refresh': () => null,
    'friend.favorite': () => null,
    'friend.remove': ({ id }: { id: string }) => {
      friends.splice(friends.findIndex((f) => f.id === id), 1);
      emit('presence.changed', { friends: structuredClone(friends) });
      return null;
    },
    'friend.block': () => null,
    'profile.get': () => structuredClone(profile),
    'profile.create': ({ name }: { name: string }) => {
      profile.name = name;
      return null;
    },
    'invite.list': () => structuredClone(invites),
    'invite.create': ({ role, uses }: { role: string; uses: number }) => {
      const now = Math.floor(Date.now() / 1000);
      invites.unshift({ tokenHash: String(now), role, createdAt: now, expiresAt: now + 86400, usesLeft: uses, needsApproval: uses > 1, active: true });
      return { link: 'falco://invite/KLUv_WBmAJUJAMZURyEA8zb8P8uPmqmBSLdemo', expiresAt: now + 86400, tailscaleShared: false };
    },
    'invite.revoke': ({ tokenHash }: { tokenHash: string }) => {
      const i = invites.find((x) => x.tokenHash === tokenHash);
      if (i) i.active = false;
      return null;
    },
    'invite.preview': () => ({
      host: { id: 'L', name: 'Luca', tag: '#AXBR-7FQZ', accent: '#3D8BDC' },
      role: 'giocatore',
      permissions: { role: 'giocatore', view: true, audio: true, gamepads: 1, keyboard: false, keyboardSystemKeys: false, mouse: false, launchApps: false, autoJoin: true, clipboard: false, endHostSession: false },
      expiresAt: Math.floor(Date.now() / 1000) + 86400,
      needsTailscaleShare: true,
    }),
    'invite.accept': () => {
      ['tailscale', 'share', 'handshake', 'pairing'].forEach((step, i) => {
        setTimeout(() => emit('invite.progress', { side: 'guest', step, ok: null }), i * 700);
        setTimeout(() => emit('invite.progress', { side: 'guest', step, ok: true }), i * 700 + 500);
      });
      return null;
    },
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
