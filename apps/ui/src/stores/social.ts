// Profilo, amici e inviti. Gli aggiornamenti arrivano dagli eventi del core.
import { reactive, readonly } from 'vue';
import {
  getTransport,
  rpc,
  type Friend,
  type InvitePreview,
  type InviteRow,
  type MyProfile,
  type Permissions,
  type Role,
} from '@/ipc';

export type InviteStep = 'tailscale' | 'share' | 'handshake' | 'pairing';

interface SocialStore {
  profile: MyProfile;
  friends: Friend[];
  invites: InviteRow[];
  /** Avanzamento dell'accettazione di un invito (lato ospite). */
  accept: { steps: Record<InviteStep, boolean | null>; error: string | null; done: boolean } | null;
  /** Richieste di ingresso in attesa (inviti a più usi). */
  requests: { requestId: string; name: string; tag: string }[];
  /** Amici che bussano per entrare nella mia sessione. */
  knocks: { friendId: string; name: string }[];
  /** Ospiti collegati alla mia sessione. */
  guests: { friendId: string; name: string; since: number }[];
  /** Inviti a giocare ricevuti. */
  playInvites: { friendId: string; name: string }[];
  /** Permessi effettivi degli ospiti collegati. */
  guestPerms: Record<string, Permissions>;
  /** Dispositivi che stanno ricevendo input dagli ospiti adesso. */
  activity: { keyboard: boolean; mouse: boolean; controller: boolean };
}

const state = reactive<SocialStore>({
  profile: { exists: false },
  friends: [],
  invites: [],
  accept: null,
  requests: [],
  knocks: [],
  guests: [],
  playInvites: [],
  guestPerms: {},
  activity: { keyboard: false, mouse: false, controller: false },
});

let activityTimer: ReturnType<typeof setTimeout> | undefined;

/** Permessi degli ospiti collegati, per le icone e il menu col tasto destro. */
export async function loadGuestPerms(): Promise<void> {
  const entries = await Promise.all(
    state.guests.map(async (g) => {
      const r = await rpc<{ effective: Permissions | null }>('friend.getPermissions', { id: g.friendId }).catch(() => null);
      return [g.friendId, r?.effective ?? null] as const;
    }),
  );
  state.guestPerms = Object.fromEntries(entries.filter((e): e is readonly [string, Permissions] => e[1] !== null));
}

export type Device = 'keyboard' | 'mouse' | 'controller';

/** Concede o toglie un dispositivo a un ospite (override sul suo ruolo). */
export async function setDevice(friendId: string, device: Device, on: boolean): Promise<void> {
  const r = await rpc<{ overrides: Record<string, unknown> | null }>('friend.getPermissions', { id: friendId });
  const key = device === 'controller' ? 'gamepads' : device;
  const overrides = { ...(r.overrides ?? {}), [key]: device === 'controller' ? (on ? 1 : 0) : on };
  await rpc('friend.setPermissions', { id: friendId, overrides });
  await loadGuestPerms();
}

/** Torna a «solo guardare». */
export async function watchOnly(friendId: string): Promise<void> {
  await rpc('friend.setPermissions', { id: friendId, role: 'spettatore', overrides: {} });
  await loadGuestPerms();
}

export async function loadProfile(): Promise<void> {
  state.profile = await rpc<MyProfile>('profile.get');
}

export async function loadFriends(): Promise<void> {
  state.friends = await rpc<Friend[]>('friends.list');
}

export async function loadInvites(): Promise<void> {
  state.invites = await rpc<InviteRow[]>('invite.list');
}

export async function initSocial(): Promise<void> {
  const t = await getTransport();
  t.onEvent((e) => {
    const p = e.params as Record<string, unknown>;
    if (e.method === 'presence.changed') {
      if (Array.isArray(p.friends)) state.friends = p.friends as Friend[];
      const self = p.self as Record<string, unknown> | undefined;
      if (self && ('profile' in self || 'hasProfile' in self)) void loadProfile();
    }
    if (e.method === 'invite.progress' && p.side === 'guest' && state.accept) {
      const step = p.step as InviteStep | 'error';
      if (step === 'error') state.accept.error = String(p.message ?? '');
      else state.accept.steps[step] = p.ok as boolean | null;
      if (step === 'pairing' && p.ok === true) state.accept.done = true;
    }
    if (e.method === 'invite.progress' && p.side === 'host') void loadInvites();
    if (e.method === 'invite.request') state.requests.push(p as SocialStore['requests'][number]);
    if (e.method === 'session.knock') state.knocks.push(p as SocialStore['knocks'][number]);
    if (e.method === 'host.guests') {
      state.guests = (p.guests as SocialStore['guests']) ?? [];
      void loadGuestPerms();
    }
    if (e.method === 'host.input') {
      state.activity = p as unknown as SocialStore['activity'];
      clearTimeout(activityTimer);
      activityTimer = setTimeout(() => (state.activity = { keyboard: false, mouse: false, controller: false }), 450);
    }
    if (e.method === 'play.invite') state.playInvites.push(p as SocialStore['playInvites'][number]);
  });
  await Promise.all([loadProfile(), loadFriends(), loadInvites()]).catch(() => {});
}

export async function createProfile(name: string): Promise<void> {
  await rpc('profile.create', { name });
  await loadProfile();
}

/** File scelto dall'utente → base64; il core ritaglia e comprime. */
export async function setAvatar(file: File): Promise<void> {
  const buf = new Uint8Array(await file.arrayBuffer());
  let bin = '';
  for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
  await rpc('profile.update', { avatarBase64: btoa(bin) });
  await loadProfile();
}

export async function rename(name: string): Promise<void> {
  await rpc('profile.update', { name });
  await loadProfile();
}

export async function createInvite(role: Role, validityHours: number, uses: number) {
  const r = await rpc<{ link: string; expiresAt: number; tailscaleShared: boolean }>('invite.create', {
    role,
    validityHours,
    uses,
  });
  await loadInvites();
  return r;
}

export async function revokeInvite(tokenHash: string): Promise<void> {
  await rpc('invite.revoke', { tokenHash });
  await loadInvites();
}

export const previewInvite = (link: string) => rpc<InvitePreview>('invite.preview', { link });

export async function acceptInvite(link: string): Promise<void> {
  state.accept = {
    steps: { tailscale: null, share: null, handshake: null, pairing: null },
    error: null,
    done: false,
  };
  await rpc('invite.accept', { link });
}

export function resetAccept(): void {
  state.accept = null;
}

export async function answerRequest(requestId: string, accept: boolean): Promise<void> {
  state.requests = state.requests.filter((r) => r.requestId !== requestId);
  await rpc('invite.approve', { requestId, accept });
}

export async function answerKnock(friendId: string, accept: boolean): Promise<void> {
  state.knocks = state.knocks.filter((k) => k.friendId !== friendId);
  await rpc('host.answerKnock', { friendId, accept });
}

export function dismissPlayInvite(friendId: string): void {
  state.playInvites = state.playInvites.filter((p) => p.friendId !== friendId);
}

export async function loadGuests(): Promise<void> {
  state.guests = (await rpc<{ guests: SocialStore['guests'] }>('host.guests')).guests;
  await loadGuestPerms();
}

export const friendApi = {
  remove: (id: string) => rpc('friend.remove', { id }),
  block: (id: string) => rpc('friend.block', { id }),
  favorite: (id: string, favorite: boolean) => rpc('friend.favorite', { id, favorite }),
  join: (friendId: string) => rpc('session.join', { friendId }),
  kick: (id: string) => rpc('host.kick', { id }),
  refresh: () => rpc('friends.refresh'),
};

/** Cerca un link di invito in un testo (appunti, messaggio incollato). */
export function findInviteLink(text: string): string | null {
  const m = /falco:\/\/invite\/[A-Za-z0-9_-]+/.exec(text);
  return m ? m[0] : null;
}

export const social = readonly(state);
