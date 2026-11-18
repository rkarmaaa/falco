// Modali dell'app: uno alla volta, apribili da qualsiasi vista o componente.
import { reactive, readonly } from 'vue';
import { acceptFriend, findInviteLink, redeemStream } from './social';
import { attempt } from './errors';
import { i18n } from '@/i18n';

const t = (key: string, args: Record<string, unknown> = {}) => i18n.global.t(key, args);

export type Modal = 'share' | 'addFriend' | 'accept' | null;

interface AcceptTarget {
  link: string | null;
  /** Parte subito, senza la scheda di anteprima (richiesta accettata, link di stream). */
  auto: boolean;
  /** Dopo l'handshake entra nella sessione dell'host. */
  joinHost: { id: string; name: string } | null;
}

const state = reactive<{ modal: Modal; accept: AcceptTarget; addFriendText: string }>({
  modal: null,
  accept: { link: null, auto: false, joinHost: null },
  addFriendText: '',
});

export const ui = readonly(state);

export function openModal(m: Exclude<Modal, 'accept' | null>, addFriendText = ''): void {
  state.addFriendText = addFriendText;
  state.modal = m;
}

export function closeModal(): void {
  state.modal = null;
}

export function openAccept(link: string | null = null, opts: Partial<Omit<AcceptTarget, 'link'>> = {}): void {
  state.accept = { link, auto: opts.auto ?? false, joinHost: opts.joinHost ?? null };
  state.modal = 'accept';
}

/** Accetta una richiesta d'amicizia: il Hub apre la rete, poi handshake e pairing come un invito. */
export async function acceptFriendRequest(id: string, name: string): Promise<void> {
  const r = await attempt(t('errors.context.request'), () => acceptFriend(id), { name });
  if (r) openAccept(r.invite, { auto: true });
}

/** Link di stream (`falco://join/…` o `https://…/j/…`): si entra subito nella sessione dell'host. */
export async function joinStream(text: string): Promise<void> {
  const r = await attempt(t('errors.context.joinLink'), () => redeemStream(text));
  if (r) openAccept(r.invite, { auto: true, joinHost: { id: r.user.id, name: r.user.name } });
}

/** Smista un link aperto dal sistema o incollato: amico, stream o invito firmato. */
export function openLink(text: string): void {
  if (/falco:\/\/join\/|\/j\/[A-Za-z2-7]{8}/.test(text)) void joinStream(text);
  else if (/falco:\/\/add\/|\/a\/[A-Za-z2-7]{8}/.test(text)) openModal('addFriend', text);
  else openAccept(findInviteLink(text) ?? text);
}
