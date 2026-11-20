<script setup lang="ts">
// Guscio dell'app: barra in alto (sezioni), al centro il contenuto o lo stream,
// in basso la barra delle azioni. A schermo intero resta solo lo stream e il pulsantino.
import { computed, onMounted, ref, watch, watchEffect } from 'vue';
import { onKeyStroke, useEventListener } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import TopBar, { type Section } from '@/components/TopBar.vue';
import ActionBar from '@/components/ActionBar.vue';
import StreamStage from '@/components/StreamStage.vue';
import StreamPill from '@/components/StreamPill.vue';
import ToastStack, { type Toast } from '@/components/ToastStack.vue';
import CoreOffline from '@/components/CoreOffline.vue';
import ShareStreamModal from '@/components/ShareStreamModal.vue';
import AddFriendModal from '@/components/AddFriendModal.vue';
import AcceptInviteModal from '@/components/AcceptInviteModal.vue';
import DebugDialog from '@/components/DebugDialog.vue';
import FriendsView from '@/views/FriendsView.vue';
import HostView from '@/views/HostView.vue';
import InvitesView from '@/views/InvitesView.vue';
import SettingsView from '@/views/SettingsView.vue';
import OnboardingView from '@/views/OnboardingView.vue';
import { clearLastSession, core, initCore, joinFriend, leaveSession, reconnectSession } from '@/stores/core';
import { findInviteLink, initSocial, previewInvite, social } from '@/stores/social';
import { attempt, errors, initErrors } from '@/stores/errors';
import { focusStream, keepStreamFocus, resetStage, setFullscreen, stage } from '@/stores/stage';
import { closeModal, joinStream, openAccept, openLink, ui } from '@/stores/ui';
import { setLocale } from '@/i18n';
import { getTransport, isTauri, rpc } from '@/ipc';
import { Radio } from 'lucide-vue-next';
import UiButton from '@/components/UiButton.vue';
import { useStreamOverlay } from '@/stores/overlay';

const { t } = useI18n();
const section = ref<Section>('friends');
const toast = ref<Toast | null>(null);
const seenLinks = new Set<string>();

const onboarding = computed(() => core.ready && core.connected && social.profileLoaded && !social.profile.exists);
const inSession = computed(() => core.session.phase !== 'idle');
const onStage = computed(() => inSession.value && section.value === 'stream');
watch(
  () => onStage.value && stage.embedded,
  (on) => keepStreamFocus(on),
  { immediate: true },
);
// Sopra lo stream incorporato gli avvisi vanno nella loro finestra trasparente
useStreamOverlay(computed(() => onStage.value && stage.embedded && !stage.fullscreen), toast, () => (toast.value = null));
const hasToasts = computed(
  () =>
    errors.toasts.length + social.knocks.length + social.requests.length + social.playInvites.length + social.friendRequests.invites.length > 0 ||
    !!toast.value,
);

watchEffect(() => setLocale(core.config?.general.language ?? 'it'));

function showToast(text: string, action?: Toast['action']): void {
  toast.value = { text, action };
  setTimeout(() => {
    if (toast.value?.text === text) toast.value = null;
  }, 7000);
}

// Mouse concesso a sessione avviata: Moonlight sceglie la modalità del mouse all'avvio, quindi
// per muoverlo (non solo cliccare) bisogna rientrare. Lo si dice, con il pulsante per farlo
watch(
  () => core.session.info?.permissions?.mouse,
  (now, before) => {
    if (now === true && before === false && core.session.phase === 'connected') {
      showToast(t('session.mouseGranted'), { label: t('session.rejoin'), run: () => void attempt(t('errors.context.join', { name: core.session.host ?? '' }), reconnectSession) });
    }
  },
);

// Collegandosi a qualcuno si va sul palco; a fine sessione si torna dove si era
let before: Section = 'friends';
/** Nome dell'host che ha chiuso lo stream: schermata di fine al posto del palco. */
const hostEnded = ref<string | null>(null);
watch(section, (s) => {
  if (s !== 'stream') hostEnded.value = null;
});
function leaveEnded(): void {
  hostEnded.value = null;
  section.value = before === 'stream' ? 'friends' : before;
}
watch(
  () => core.session.phase,
  (phase, prev) => {
    if (prev === 'idle' && phase !== 'idle') {
      if (section.value !== 'stream') before = section.value;
      section.value = 'stream';
    }
    if (phase === 'idle' && prev !== 'idle') {
      // Cambio di qualità: la sessione si riapre subito, niente avviso né cambio di sezione
      if (core.reconnecting) return;
      void resetStage();
      // L'host ha chiuso lo stream: si resta sulla scheda con la schermata di fine, senza errori
      if (core.lastSession?.reason === 'host-ended') {
        hostEnded.value = core.lastSession.host;
        section.value = 'stream';
        clearLastSession();
        return;
      }
      if (section.value === 'stream') section.value = before;
      sessionEnded();
    }
  },
);

function sessionEnded(): void {
  const last = core.lastSession;
  if (!last) return;
  const left = last.reason === 'left' || last.reason === '0' || last.reason === 'closed';
  const text = left ? t('session.endedLeft', { name: last.host }) : t('session.endedLost', { name: last.host });
  const friendId = last.friendId;
  showToast(
    text,
    friendId && !left ? { label: t('session.rejoin'), run: () => void attempt(t('errors.context.join', { name: last.host }), () => joinFriend(friendId, last.host)) } : undefined,
  );
  clearLastSession();
}

// Esc esce dallo schermo intero quando la tastiera è sulla nostra pagina
onKeyStroke('Escape', () => {
  if (stage.fullscreen) void setFullscreen(false);
});

// Molte chat non rendono cliccabile falco://: il percorso principale sono gli appunti (§6)
async function checkClipboard(): Promise<void> {
  if (!isTauri() || !social.profile.exists || ui.modal || inSession.value) return;
  try {
    const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
    const text = (await readText()) ?? '';
    const stream = /https?:\/\/\S+\/j\/[A-Za-z2-7]{8}|falco:\/\/join\/[A-Za-z2-7]{8}/.exec(text)?.[0];
    if (stream && !seenLinks.has(stream)) {
      seenLinks.add(stream);
      showToast(t('stream.clipboard'), { label: t('stream.watch'), run: () => void joinStream(stream) });
      return;
    }
    const link = findInviteLink(text);
    if (!link || seenLinks.has(link)) return;
    seenLinks.add(link);
    const p = await previewInvite(link);
    if (p.host.id === social.profile.id) return;
    showToast(t('accept.clipboard', { name: p.host.name }), { label: t('accept.open'), run: () => openAccept(link) });
  } catch {
    // Appunti vuoti o link scaduto: niente da mostrare
  }
}
useEventListener(window, 'focus', () => {
  void checkClipboard();
  // Con lo stream incorporato la tastiera va a Moonlight, non alla pagina
  if (onStage.value) focusStream();
});

onMounted(async () => {
  void initErrors();
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const s = await invoke<string | null>('initial_section').catch(() => null);
    if (s) section.value = s as Section;
    const l = await invoke<string | null>('initial_link').catch(() => null);
    if (l) openLink(l);
    const { listen } = await import('@tauri-apps/api/event');
    // Seconda apertura di Falco: la finestra esistente riceve link e sezione
    void listen<string>('ui://link', (e) => openLink(e.payload));
    void listen<string>('ui://section', (e) => (section.value = e.payload as Section));
    void listen<string>('pill://action', (e) => {
      if (e.payload === 'exit') void setFullscreen(false);
      if (e.payload === 'leave') void attempt(t('errors.context.leave'), () => leaveSession());
    });
    // Chiudere la finestra durante uno stream: prima si esce dalla sessione
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    void getCurrentWindow().onCloseRequested(async () => {
      if (core.session.phase !== 'idle') {
        await leaveSession().catch(() => {});
        // Un errore qui non deve impedire la chiusura
        await resetStage().catch(() => {});
      }
      // Fuori dallo schermo intero prima di chiudere: si ricorda la dimensione normale
      if (stage.fullscreen) await setFullscreen(false).catch(() => {});
    });
  }
  const transport = await getTransport();
  transport.onEvent((e) => {
    if (e.method === 'ui.navigate') section.value = (e.params as { section: Section }).section;
    if (e.method === 'ui.openLink') openLink((e.params as { link: string }).link);
    if (e.method === 'update.available') {
      const v = (e.params as { version: string }).version;
      showToast(t('update.available', { version: v }), { label: t('update.install'), run: () => void attempt(t('errors.context.update'), () => rpc('update.install')) });
    }
  });
  await initCore();
  await initSocial();
  if (core.session.phase !== 'idle') section.value = 'stream';
  void checkClipboard();
});
</script>

<template>
  <div class="app" :class="{ fullscreen: stage.fullscreen, onboarding }">
    <TopBar v-if="!stage.fullscreen" v-model="section" :requests="social.requests.length" :friend-requests="social.friendRequests.incoming.length" />

    <main v-if="onboarding" class="content">
      <OnboardingView />
    </main>

    <div v-else class="body">
      <div v-if="onStage" class="stage-row">
        <StreamStage />
        <!-- Nel browser (simulazione) gli avvisi restano accanto al palco -->
        <ToastStack v-if="hasToasts && !stage.fullscreen && !isTauri()" docked :toast="toast" @clear="toast = null" />
      </div>
      <template v-else>
        <main class="content">
          <CoreOffline v-if="core.ready && !core.connected" />
          <div class="view">
            <section v-if="section === 'stream' && !inSession && hostEnded" class="ended">
              <span class="icon"><Radio :size="28" :stroke-width="1.6" aria-hidden="true" /></span>
              <h1>{{ t('session.hostEnded.title', { name: hostEnded }) }}</h1>
              <p>{{ t('session.hostEnded.body') }}</p>
              <UiButton variant="primary" :size="48" @click="leaveEnded">{{ t('session.hostEnded.back') }}</UiButton>
            </section>
            <FriendsView v-else-if="section === 'friends' || (section === 'stream' && !inSession)" />
            <HostView v-else-if="section === 'host'" />
            <InvitesView v-else-if="section === 'invites'" />
            <SettingsView v-else-if="section === 'settings'" />
          </div>
        </main>
        <div id="detail-slot" />
      </template>
    </div>

    <ActionBar v-if="!onboarding && !stage.fullscreen" @navigate="section = $event as Section" />

    <!-- Nel browser il pulsantino è simulato dentro la pagina -->
    <div v-if="stage.fullscreen && !isTauri()" class="pill-dev">
      <StreamPill :name="core.session.host ?? ''" :stats="core.session.stats" @exit="setFullscreen(false)" @leave="leaveSession()" />
    </div>

    <ShareStreamModal v-if="ui.modal === 'share'" @close="closeModal" />
    <AddFriendModal v-if="ui.modal === 'addFriend'" @close="closeModal" />
    <AcceptInviteModal v-if="ui.modal === 'accept'" :key="ui.accept.link ?? ''" @close="closeModal" />
    <DebugDialog v-if="errors.debug" :error="errors.debug" />
    <ToastStack v-if="!onStage && !stage.fullscreen" :toast="toast" @clear="toast = null" />
  </div>
</template>

<style lang="scss" scoped>
.app {
  display: grid;
  grid-template-rows: $topbar-h 1fr $actionbar-h;
  height: 100%;

  &.onboarding {
    grid-template-rows: $topbar-h 1fr;
  }

  &.fullscreen {
    grid-template-rows: 1fr;
    background: #000;
  }

  .body {
    display: flex;
    min-height: 0;
  }

  .stage-row {
    display: flex;
    flex: 1;
    gap: $sp-3;
    min-width: 0;
    padding: $sp-3;

    > :first-child {
      flex: 1;
      min-width: 0;
    }
  }

  &.fullscreen .stage-row {
    padding: 0;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;

    .view {
      max-width: $page-max-w;
      margin: 0 auto;
      padding: $page-pad-y $page-pad-x $page-pad-x;

      @include mq(sm) {
        padding: $page-pad-sm;
      }
    }
  }

  #detail-slot {
    display: contents;
  }

  .pill-dev {
    position: fixed;
    top: 12px;
    left: 50%;
    z-index: 60;
    transform: translateX(-50%);
  }
}

.ended {
  display: grid;
  justify-items: center;
  gap: $sp-3;
  max-width: 440px;
  margin: $sp-9 auto 0;
  text-align: center;

  .icon {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: $r-pill;
    background: $bg-3;
    color: $text-2;
  }

  h1 {
    @include title-h1;
  }

  p {
    margin-bottom: $sp-3;
    color: $text-2;
  }
}
</style>
