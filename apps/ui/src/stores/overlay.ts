// Notifiche sopra lo stream: con il video incorporato la pagina di Falco sta sotto la finestra di
// Moonlight, quindi le notifiche vanno in una finestra a parte (ToastsApp) in basso a destra del
// palco. Qui si prepara il contenuto, si posiziona la finestra e si eseguono i clic.
import { computed, watch, type Ref } from 'vue';
import { i18n } from '@/i18n';
import { isTauri } from '@/ipc';
import { answerKnock, answerRequest, declineInvite, dismissPlayInvite, social } from './social';
import { attempt, dismiss, errors, openDebug } from './errors';
import { joinFriend } from './core';
import { acceptDirectInvite } from './ui';
import type { OverlayItem } from '@/ToastsApp.vue';

const t = (key: string, args: Record<string, unknown> = {}) => i18n.global.t(key, args);
const WIDTH = 380;
const MARGIN = 12;

export interface PlainToast {
  text: string;
  action?: { label: string; run: () => void };
}

/** Collega la finestra delle notifiche finché `active` è vero (stream incorporato in Tauri). */
export function useStreamOverlay(active: Ref<boolean>, message: Ref<PlainToast | null>, clearMessage: () => void): void {
  if (!isTauri()) return;
  let height = 0;
  let open = false;

  const friend = (id: string) => social.friends.find((f) => f.id === id);
  const items = computed<OverlayItem[]>(() => [
    ...social.knocks.map((k) => ({
      kind: 'knock' as const,
      id: k.friendId,
      name: k.name,
      text: t('knock.wants', { name: k.name }),
      avatar: friend(k.friendId)?.avatarUrl,
      accent: friend(k.friendId)?.accent,
      no: t('requests.decline'),
      yes: t('requests.accept'),
    })),
    ...social.playInvites.map((p) => ({
      kind: 'play' as const,
      id: p.friendId,
      name: p.name,
      text: t('knock.invited', { name: p.name }),
      avatar: friend(p.friendId)?.avatarUrl,
      accent: friend(p.friendId)?.accent,
      no: t('knock.later'),
      yes: t('friends.join'),
    })),
    ...social.friendRequests.invites.map((i) => ({
      kind: 'invite' as const,
      id: i.id,
      name: i.user.name,
      text: t('knock.invited', { name: i.user.name }),
      // Il Hub usa l'id dell'identità: è lo stesso dell'amico salvato qui
      avatar: friend(i.user.id)?.avatarUrl,
      accent: friend(i.user.id)?.accent,
      no: t('requests.decline'),
      yes: t('friends.join'),
    })),
    ...social.requests.map((r) => ({
      kind: 'request' as const,
      id: r.requestId,
      name: r.name,
      text: `${t('requests.wants', { name: r.name })} ${r.tag}`,
      no: t('requests.decline'),
      yes: t('requests.accept'),
    })),
    ...errors.toasts.map((e) => ({ kind: 'error' as const, id: String(e.id), text: e.friendly, yes: t('errors.debug') })),
    ...(message.value ? [{ kind: 'msg' as const, id: 'msg', text: message.value.text, yes: message.value.action?.label }] : []),
  ]);

  async function place(): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    const stage = document.querySelector('.stage')?.getBoundingClientRect();
    const show = active.value && items.value.length > 0 && !!stage;
    if (!show || !stage) {
      if (open) await invoke('toasts_window', { show: false, x: 0, y: 0, w: 0, h: 0 }).catch(() => {});
      open = false;
      return;
    }
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    const [pos, scale] = await Promise.all([win.innerPosition(), win.scaleFactor()]);
    const h = Math.max(height, 64);
    const x = pos.x / scale + stage.right - WIDTH - MARGIN;
    const y = pos.y / scale + stage.bottom - h - MARGIN;
    await invoke('toasts_window', { show: true, x, y, w: WIDTH, h }).catch(() => {});
    open = true;
  }

  async function push(): Promise<void> {
    const { emitTo } = await import('@tauri-apps/api/event');
    await emitTo('toasts', 'toasts://state', items.value).catch(() => {});
  }

  watch([items, active], async () => {
    await place();
    await push();
  }, { deep: true });

  void (async () => {
    const { listen } = await import('@tauri-apps/api/event');
    await listen<number>('toasts://height', (e) => {
      height = e.payload;
      void place();
    });
    await listen('toasts://ready', () => void push());
    await listen<{ kind: OverlayItem['kind']; id: string; ok: boolean }>('toasts://action', (e) => void act(e.payload));
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().onMoved(() => void place());
    await getCurrentWindow().onResized(() => void place());
  })();

  async function act({ kind, id, ok }: { kind: OverlayItem['kind']; id: string; ok: boolean }): Promise<void> {
    const ctxRequest = t('errors.context.request');
    switch (kind) {
      case 'knock':
        await attempt(t('errors.context.knock'), () => answerKnock(id, ok));
        break;
      case 'request':
        await attempt(ctxRequest, () => answerRequest(id, ok));
        break;
      case 'play': {
        const name = social.playInvites.find((p) => p.friendId === id)?.name ?? '';
        dismissPlayInvite(id);
        if (ok) await attempt(t('errors.context.join', { name }), () => joinFriend(id, name), { name });
        break;
      }
      case 'invite': {
        const name = social.friendRequests.invites.find((i) => i.id === id)?.user.name ?? '';
        if (ok) await acceptDirectInvite(id, name);
        else await attempt(ctxRequest, () => declineInvite(id));
        break;
      }
      case 'error': {
        const err = errors.toasts.find((e) => String(e.id) === id);
        if (err && ok) openDebug(err);
        if (err) dismiss(err.id);
        break;
      }
      case 'msg':
        if (ok) message.value?.action?.run();
        clearMessage();
        break;
    }
  }
}
