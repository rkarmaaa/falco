// Profilo, amici e inviti. Gli aggiornamenti arrivano dagli eventi del core.
import { reactive, readonly } from 'vue';
import {
  getTransport,
  rpc,
  type Friend,
  type InvitePreview,
  type InviteRow,
  type MyProfile,
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
});

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
    if (e.method === 'host.guests') state.guests = (p.guests as SocialStore['guests']) ?? [];
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
