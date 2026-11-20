// Modali dell'app: uno alla volta, apribili da qualsiasi vista o componente.
import { reactive, readonly } from 'vue';
import { acceptFinished, acceptFriend, acceptInvite, findInviteLink, redeemStream, resetAccept, social, takeInvite } from './social';
import { beginLink, endLink, joinFriend } from './core';
import { attempt, reportError } from './errors';
import { isRpcError } from '@/ipc';
import { i18n } from '@/i18n';

const t = (key: string, args: Record<string, unknown> = {}) => i18n.global.t(key, args);

export type Modal = 'share' | 'addFriend' | 'accept' | null;

interface AcceptTarget {
  link: string | null;
  /** Parte subito, senza la scheda di anteprima (richiesta accettata, link di stream). */
  auto: boolean;
}

const state = reactive<{ modal: Modal; accept: AcceptTarget; addFriendText: string }>({
  modal: null,
  accept: { link: null, auto: false },
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
  state.accept = { link, auto: opts.auto ?? false };
  state.modal = 'accept';
}

/** Accetta una richiesta d'amicizia: il Hub apre la rete, poi handshake e pairing come un invito. */
export async function acceptFriendRequest(id: string, name: string): Promise<void> {
  const r = await attempt(t('errors.context.request'), () => acceptFriend(id), { name });
  if (r) openAccept(r.invite, { auto: true });
}

type Host = { id: string; name: string };

/**
 * Invito o link di stream di un host: palco subito, con la sua foto e i passi. Dietro si collegano
 * i due PC (handshake e pairing, che registrano anche il nodo Tailscale attuale) e poi si entra.
 * Una sola schermata dall'inizio alla fine, nessun popup in mezzo.
 */
async function linkAndJoin(invite: string, host: Host): Promise<void> {
  const context = t('errors.context.join', { name: host.name });
  beginLink(host.id, host.name);
  try {
    await acceptInvite(invite);
    await acceptFinished();
  } catch (e) {
    endLink();
    reportError(e, context, { name: host.name });
    return;
  } finally {
    resetAccept();
  }
  await attempt(context, () => joinFriend(host.id, host.name), { name: host.name });
}

/** Errore di entrata che un nuovo collegamento può risolvere (nodo Tailscale cambiato, pairing perso). */
function needsRelink(e: unknown): boolean {
  const reason = isRpcError(e) ? (e.data as { reason?: unknown } | undefined)?.reason : undefined;
  return reason === 'offline' || reason === 'unrecognized';
}

/** Entra da un amico già associato; se non lo raggiunge, rifà il collegamento con l'invito. */
async function joinOrRelink(invite: string, host: Host): Promise<void> {
  if (social.friends.some((f) => f.id === host.id)) {
    try {
      await joinFriend(host.id, host.name);
      return;
    } catch (e) {
      if (!needsRelink(e)) {
        reportError(e, t('errors.context.join', { name: host.name }), { name: host.name });
        return;
      }
    }
  }
  await linkAndJoin(invite, host);
}

/** Invito diretto di un amico: si entra subito nella sua sessione. */
export async function acceptDirectInvite(id: string, name: string): Promise<void> {
  const r = await attempt(t('errors.context.join', { name }), () => takeInvite(id), { name });
  if (r) await joinOrRelink(r.invite, { id: r.user.id, name: r.user.name });
}

/** Link di stream (`falco://join/…` o `https://…/j/…`): si entra subito nella sessione dell'host. */
export async function joinStream(text: string): Promise<void> {
  closeModal();
  const r = await attempt(t('errors.context.joinLink'), () => redeemStream(text));
  if (r) await joinOrRelink(r.invite, { id: r.user.id, name: r.user.name });
}

/** Smista un link aperto dal sistema o incollato: amico, stream o invito firmato. */
export function openLink(text: string): void {
  if (/falco:\/\/join\/|\/j\/[A-Za-z2-7]{8}/.test(text)) void joinStream(text);
  else if (/falco:\/\/add\/|\/a\/[A-Za-z2-7]{8}/.test(text)) openModal('addFriend', text);
  else openAccept(findInviteLink(text) ?? text);
}
