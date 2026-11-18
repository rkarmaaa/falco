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
import InviteModal from '@/components/InviteModal.vue';
import AcceptInviteModal from '@/components/AcceptInviteModal.vue';
import DebugDialog from '@/components/DebugDialog.vue';
import FriendsView from '@/views/FriendsView.vue';
import HostView from '@/views/HostView.vue';
import InvitesView from '@/views/InvitesView.vue';
import SettingsView from '@/views/SettingsView.vue';
import OnboardingView from '@/views/OnboardingView.vue';
import { clearLastSession, core, initCore, joinFriend, leaveSession } from '@/stores/core';
import { findInviteLink, initSocial, previewInvite, social } from '@/stores/social';
import { attempt, errors, initErrors } from '@/stores/errors';
import { resetStage, setFullscreen, stage } from '@/stores/stage';
import { setLocale } from '@/i18n';
import { getTransport, isTauri, rpc } from '@/ipc';

const { t } = useI18n();
const section = ref<Section>('friends');
const modal = ref<'invite' | 'accept' | null>(null);
const acceptLink = ref<string | null>(null);
const toast = ref<Toast | null>(null);
const seenLinks = new Set<string>();

const onboarding = computed(() => core.ready && core.connected && !social.profile.exists);
const inSession = computed(() => core.session.phase !== 'idle');
const onStage = computed(() => inSession.value && section.value === 'stream');
const hasToasts = computed(
  () => errors.toasts.length + social.knocks.length + social.requests.length + social.playInvites.length > 0 || !!toast.value,
);

watchEffect(() => setLocale(core.config?.general.language ?? 'it'));

function showToast(text: string, action?: Toast['action']): void {
  toast.value = { text, action };
  setTimeout(() => {
    if (toast.value?.text === text) toast.value = null;
  }, 7000);
}

function openAccept(link: string | null = null): void {
  acceptLink.value = link;
  modal.value = 'accept';
}

// Collegandosi a qualcuno si va sul palco; a fine sessione si torna dove si era
let before: Section = 'friends';
watch(
  () => core.session.phase,
  (phase, prev) => {
    if (prev === 'idle' && phase !== 'idle') {
      if (section.value !== 'stream') before = section.value;
      section.value = 'stream';
    }
    if (phase === 'idle' && prev !== 'idle') {
      void resetStage();
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
  if (!isTauri() || !social.profile.exists || modal.value || inSession.value) return;
  try {
    const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
    const link = findInviteLink((await readText()) ?? '');
    if (!link || seenLinks.has(link)) return;
    seenLinks.add(link);
    const p = await previewInvite(link);
    if (p.host.id === social.profile.id) return;
    showToast(t('accept.clipboard', { name: p.host.name }), { label: t('accept.open'), run: () => openAccept(link) });
  } catch {
    // Appunti vuoti o link scaduto: niente da mostrare
  }
}
useEventListener(window, 'focus', () => void checkClipboard());

onMounted(async () => {
  void initErrors();
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const s = await invoke<string | null>('initial_section').catch(() => null);
    if (s) section.value = s as Section;
    const l = await invoke<string | null>('initial_link').catch(() => null);
    if (l) openAccept(l);
    const { listen } = await import('@tauri-apps/api/event');
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
    });
  }
  const transport = await getTransport();
  transport.onEvent((e) => {
    if (e.method === 'ui.navigate') section.value = (e.params as { section: Section }).section;
    if (e.method === 'ui.openLink') openAccept((e.params as { link: string }).link);
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
    <TopBar v-if="!stage.fullscreen" v-model="section" :requests="social.requests.length" />

    <main v-if="onboarding" class="content">
      <OnboardingView />
    </main>

    <div v-else class="body">
      <div v-if="onStage" class="stage-row">
        <StreamStage />
        <ToastStack v-if="hasToasts && !stage.fullscreen" docked :toast="toast" @clear="toast = null" />
      </div>
      <template v-else>
        <main class="content">
          <CoreOffline v-if="core.ready && !core.connected" />
          <div class="view">
            <FriendsView v-if="section === 'friends' || (section === 'stream' && !inSession)" @invite="modal = 'invite'" @accept="openAccept()" />
            <HostView v-else-if="section === 'host'" @invite="modal = 'invite'" />
            <InvitesView v-else-if="section === 'invites'" @invite="modal = 'invite'" />
            <SettingsView v-else-if="section === 'settings'" />
          </div>
        </main>
        <div id="detail-slot" />
      </template>
    </div>

    <ActionBar v-if="!onboarding && !stage.fullscreen" @invite="modal = 'invite'" @navigate="section = $event as Section" />

    <!-- Nel browser il pulsantino è simulato dentro la pagina -->
    <div v-if="stage.fullscreen && !isTauri()" class="pill-dev">
      <StreamPill :name="core.session.host ?? ''" :stats="core.session.stats" @exit="setFullscreen(false)" @leave="leaveSession()" />
    </div>

    <InviteModal v-if="modal === 'invite'" @close="modal = null" />
    <AcceptInviteModal v-if="modal === 'accept'" :initial-link="acceptLink" @close="modal = null" />
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
      max-width: 1120px;
      margin: 0 auto;
      padding: $sp-7 $sp-8 $sp-8;

      @include mq(sm) {
        padding: $sp-6;
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
</style>
