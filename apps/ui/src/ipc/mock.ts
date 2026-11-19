// Core simulato per il browser: stesso contratto JSON-RPC, dati finti.
import type { Transport } from './index';
import { NOT_IMPLEMENTED, type CoreEvent, type StateSnapshot } from './types';

export function createMockTransport(): Transport {
  const listeners: ((e: CoreEvent) => void)[] = [];
  const emit = (method: string, params: unknown) => listeners.forEach((cb) => cb({ method, params }));

  const state: StateSnapshot = {
    version: '1.0.0-rc.7',
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
    general: { language: 'it', updates: 'auto', start_with_system: true, start_minimized: true, close_to_tray: true },
    profile: { status: 'online' },
    stream: {},
    host: { max_guests: 3, encoder: 'auto', default_role: 'spettatore', profile: 'latency', max_bitrate_kbps: 0, encoder_quality: 'speed', audio_capture: 'onscreen' },
    network: { provider: 'tailscale', presence_port: 47900, fec_percent: 20 },
    performance: {},
    advanced: {},
  } as Record<string, Record<string, unknown>>;

  // Ospiti simulati: con ?guests nella URL entrano subito, altrimenti dopo che l'host è pronto
  const guests: { friendId: string; name: string; since: number }[] = [];
  const perms: Record<string, Record<string, unknown>> = {
    L: { role: 'spettatore', keyboard: false, mouse: false, gamepads: 0 },
    B: { role: 'spettatore', keyboard: false, mouse: false, gamepads: 0 },
  };
  const effective = (id: string) => {
    const p = perms[id] ?? { role: 'spettatore', keyboard: false, mouse: false, gamepads: 0 };
    return { role: p.role, view: true, audio: true, gamepads: p.gamepads, keyboard: p.keyboard, keyboardSystemKeys: false, mouse: p.mouse, launchApps: false, autoJoin: false, clipboard: false, endHostSession: false };
  };
  let inputTimer: ReturnType<typeof setInterval> | undefined;
  function simulateGuests(): void {
    const now = Math.floor(Date.now() / 1000);
    setTimeout(() => {
      guests.splice(0, guests.length, { friendId: 'L', name: 'Luca', since: now - 300 }, { friendId: 'B', name: 'Bea', since: now - 60 });
      emit('host.guests', { guests: structuredClone(guests) });
    }, 900);
    clearInterval(inputTimer);
    // Input a raffiche, come un ospite che gioca
    inputTimer = setInterval(() => {
      if (!guests.length) return;
      const r = Math.random();
      emit('host.input', { keyboard: r < 0.5, mouse: r > 0.3, controller: r > 0.8 });
    }, 700);
  }
  let sessionTimers: ReturnType<typeof setTimeout>[] = [];
  function startSession(name: string, friendId: string | null): void {
    sessionTimers.forEach(clearTimeout);
    const started = Math.floor(Date.now() / 1000);
    const info = { host: '100.64.0.2', friendId, name, pid: 4242, startedAt: started, permissions: { ...effective('X'), gamepads: 1 }, app: 'Desktop' };
    state.streaming = true;
    state.session = info as StateSnapshot['session'];
    sessionTimers = [
      setTimeout(() => emit('session.started', info), 600),
      setTimeout(() => emit('session.stats', { phase: 'connecting' }), 900),
      setTimeout(() => emit('session.stats', { phase: 'connected' }), 1800),
    ];
    let n = 0;
    const tick = () => {
      if (!state.streaming) return;
      emit('session.stats', { phase: 'stats', rttMs: 9 + Math.random() * 6, fps: 59 + Math.random(), bitrateKbps: 38000 + Math.random() * 6000, path: 'direct' });
      if (++n < 10_000) sessionTimers.push(setTimeout(tick, 1000));
    };
    sessionTimers.push(setTimeout(tick, 2400));
  }
  function endSession(reason: string): void {
    sessionTimers.forEach(clearTimeout);
    if (!state.streaming) return;
    state.streaming = false;
    state.session = null;
    emit('session.ended', { host: '100.64.0.2', durationS: 42, reason, latencyMs: 12.4 });
  }

  let hubRequests = {
    incoming: [{ id: 'r1', user: { id: 'G', name: 'Giulia', code: 'GQ4M2ZTA' }, createdAt: Math.floor(Date.now() / 1000) - 600 }],
    outgoing: [] as { id: string; user: { id: string; name: string; code: string }; createdAt: number }[],
    invites: [] as { id: string; user: { id: string; name: string; code: string }; createdAt: number }[],
  };

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
    'update.check': () => ({ available: { version: '1.0.0', notes: 'Prima versione stabile.' }, current: '1.0.0-rc.7', enabled: true }),
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
          if (location.search.includes('guests')) simulateGuests();
        }, 1500);
      else {
        guests.splice(0);
        emit('host.guests', { guests: [] });
      }
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
    'session.join': ({ host, friendId }: { host?: string; friendId?: string }) => {
      const f = friends.find((x) => x.id === friendId);
      startSession(f?.name ?? host ?? '?', friendId ?? null);
      return null;
    },
    'session.leave': () => {
      endSession('left');
      return null;
    },
    'host.guests': () => ({ guests: structuredClone(guests) }),
    'host.kick': ({ id }: { id: string }) => {
      guests.splice(guests.findIndex((g) => g.friendId === id), 1);
      emit('host.guests', { guests: structuredClone(guests) });
      return null;
    },
    'host.applyInputs': () => null,
    'host.recommend': async () => {
      await new Promise((r) => setTimeout(r, 1800));
      return { profile: 'balanced', reasons: ['mediumBandwidth', 'wifi'], checks: [] };
    },
    'friend.getPermissions': ({ id }: { id: string }) => ({ role: perms[id]?.role ?? 'spettatore', overrides: { ...(perms[id] ?? {}) }, effective: effective(id), paired: true }),
    'friend.setPermissions': ({ id, role, overrides }: { id: string; role?: string; overrides: Record<string, unknown> }) => {
      perms[id] = { ...(role ? { role, keyboard: false, mouse: false, gamepads: 0 } : perms[id]), ...overrides };
      return null;
    },
    'debug.report': () => ({ app: { version: state.version }, errors: [], logs: { core: ['(simulato)'] } }),
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
    'hub.info': () => ({ configured: true, code: 'K7QM-2XPA', friendLink: 'https://hub.falco.test/a/K7QM2XPA', network: true }),
    'hub.requests': () => structuredClone(hubRequests),
    'hub.friends': () => [{ id: 'M', name: 'MANUZZO', code: '7UWUUTTW' }],
    'hub.inviteFriend': () => null,
    'hub.takeInvite': () => ({ invite: 'falco://invite/KLUv_WBmAJUJAMZURyEA8zb8P8uPmqmBSLdemo', user: { id: 'L', name: 'Luca', code: 'AXBR7FQZ' } }),
    'hub.declineInvite': () => null,
    'hub.addFriend': ({ text }: { text: string }) => {
      if (!/[A-Z2-7]{4}-?[A-Z2-7]{4}/i.test(text)) throw { code: -32602, message: 'codice amico non valido: sono 8 caratteri, come K7QM-2XPA' };
      const user = { id: 'N', name: 'Nico', code: text.toUpperCase().replace(/[^A-Z2-7]/g, '').slice(-8) };
      hubRequests.outgoing.unshift({ id: String(Date.now()), user, createdAt: Math.floor(Date.now() / 1000) });
      return { id: 'x', user };
    },
    'hub.accept': ({ id }: { id: string }) => {
      const r = hubRequests.incoming.find((x) => x.id === id);
      hubRequests.incoming = hubRequests.incoming.filter((x) => x.id !== id);
      return { invite: 'falco://invite/KLUv_WBmAJUJAMZURyEA8zb8P8uPmqmBSLdemo', user: r?.user };
    },
    'hub.decline': ({ id }: { id: string }) => {
      hubRequests.incoming = hubRequests.incoming.filter((x) => x.id !== id);
      hubRequests.outgoing = hubRequests.outgoing.filter((x) => x.id !== id);
      return null;
    },
    'hub.shareStream': () => ({ code: 'QW3R7YUP', url: 'https://hub.falco.test/j/QW3R7YUP', expiresAt: Math.floor(Date.now() / 1000) + 86400 }),
    'hub.join': () => ({ invite: 'falco://invite/KLUv_WBmAJUJAMZURyEA8zb8P8uPmqmBSLdemo', user: { id: 'L', name: 'Luca', code: 'AXBR7FQZ' } }),
    'net.joinFalco': () => null,
    'app.openWeb': () => null,
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
      return (await h(params ?? {})) as T;
    },
    onEvent: (cb) => void listeners.push(cb),
    onConnection: () => {},
    connected: async () => true,
  };
}
