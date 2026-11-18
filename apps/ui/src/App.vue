<script setup lang="ts">
import { onMounted, ref, watchEffect } from 'vue';
import { useEventListener, usePreferredDark } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import TitleBar from '@/components/TitleBar.vue';
import SideBar, { type Section } from '@/components/SideBar.vue';
import CoreOffline from '@/components/CoreOffline.vue';
import UiButton from '@/components/UiButton.vue';
import InviteModal from '@/components/InviteModal.vue';
import AcceptInviteModal from '@/components/AcceptInviteModal.vue';
import FriendsView from '@/views/FriendsView.vue';
import HostView from '@/views/HostView.vue';
import InvitesView from '@/views/InvitesView.vue';
import SettingsView from '@/views/SettingsView.vue';
import OnboardingView from '@/views/OnboardingView.vue';
import { core, initCore } from '@/stores/core';
import { answerKnock, answerRequest, dismissPlayInvite, findInviteLink, initSocial, previewInvite, social } from '@/stores/social';
import { joinFriend } from '@/stores/core';
import { setLocale } from '@/i18n';
import { getTransport, isTauri } from '@/ipc';

const { t } = useI18n();
const section = ref<Section>('friends');
const modal = ref<'invite' | 'accept' | null>(null);
const acceptLink = ref<string | null>(null);
const toast = ref<{ text: string; action?: { label: string; run: () => void } } | null>(null);
const prefersDark = usePreferredDark();
const seenLinks = new Set<string>();

// Tema e lingua seguono config.toml; «system» segue il sistema operativo
watchEffect(() => {
  const theme = core.config?.general.theme ?? 'system';
  const dark = theme === 'dark' || (theme === 'system' && prefersDark.value);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  setLocale(core.config?.general.language ?? 'it');
});

function showToast(text: string, action?: { label: string; run: () => void }): void {
  toast.value = { text, action };
  setTimeout(() => {
    if (toast.value?.text === text) toast.value = null;
  }, 6000);
}

function openAccept(link: string | null = null): void {
  acceptLink.value = link;
  modal.value = 'accept';
}

// Molte chat non rendono cliccabile falco://: il percorso principale sono gli appunti (§6)
async function checkClipboard(): Promise<void> {
  if (!isTauri() || !social.profile.exists || modal.value) return;
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
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const s = await invoke<string | null>('initial_section').catch(() => null);
    if (s) section.value = s as Section;
  }
  const t = await getTransport();
  t.onEvent((e) => {
    if (e.method === 'ui.navigate') section.value = (e.params as { section: Section }).section;
  });
  await initCore();
  await initSocial();
  void checkClipboard();
});
</script>

<template>
  <div class="app">
    <TitleBar />
    <div class="body">
      <template v-if="core.ready && !social.profile.exists && core.connected">
        <main class="content">
          <OnboardingView />
        </main>
      </template>
      <template v-else>
        <SideBar v-model="section" />
        <main class="content">
          <CoreOffline v-if="core.ready && !core.connected" />
          <div class="view">
            <FriendsView v-if="section === 'friends'" @invite="modal = 'invite'" @accept="openAccept()" @error="showToast" />
            <HostView v-else-if="section === 'host'" />
            <InvitesView v-else-if="section === 'invites'" @invite="modal = 'invite'" />
            <SettingsView v-else />
          </div>
        </main>
        <div id="detail-slot" />
      </template>
    </div>

    <InviteModal v-if="modal === 'invite'" @close="modal = null" />
    <AcceptInviteModal v-if="modal === 'accept'" :initial-link="acceptLink" @close="modal = null" />

    <div class="toasts" aria-live="polite">
      <div v-for="k in social.knocks" :key="k.friendId" class="toast request" role="alert">
        <span>{{ $t('knock.wants', { name: k.name }) }}</span>
        <UiButton variant="primary" :size="32" @click="answerKnock(k.friendId, true)">{{ $t('requests.accept') }}</UiButton>
        <UiButton variant="ghost" :size="32" @click="answerKnock(k.friendId, false)">{{ $t('requests.decline') }}</UiButton>
      </div>
      <div v-for="p in social.playInvites" :key="p.friendId" class="toast request" role="alert">
        <span>{{ $t('knock.invited', { name: p.name }) }}</span>
        <UiButton
          variant="primary"
          :size="32"
          @click="
            dismissPlayInvite(p.friendId);
            joinFriend(p.friendId, p.name).catch((e) => showToast(String(e?.message ?? e)));
          "
        >
          {{ $t('friends.join') }}
        </UiButton>
        <UiButton variant="ghost" :size="32" @click="dismissPlayInvite(p.friendId)">{{ $t('knock.later') }}</UiButton>
      </div>
      <div v-for="r in social.requests" :key="r.requestId" class="toast request" role="alert">
        <span>{{ $t('requests.wants', { name: r.name }) }} <span class="tag">{{ r.tag }}</span></span>
        <UiButton variant="primary" :size="32" @click="answerRequest(r.requestId, true)">{{ $t('requests.accept') }}</UiButton>
        <UiButton variant="ghost" :size="32" @click="answerRequest(r.requestId, false)">{{ $t('requests.decline') }}</UiButton>
      </div>
      <Transition name="toast">
        <div v-if="toast" class="toast">
          <span>{{ toast.text }}</span>
          <UiButton
            v-if="toast.action"
            variant="secondary"
            :size="32"
            @click="
              toast.action.run();
              toast = null;
            "
          >
            {{ toast.action.label }}
          </UiButton>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.app {
  display: grid;
  grid-template-rows: $titlebar-h 1fr;
  height: 100%;

  .body {
    display: flex;
    min-height: 0;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;

    .view {
      padding: $sp-7 $sp-8;

      @include mq(sm) {
        padding: $sp-6;
      }
    }
  }

  #detail-slot {
    display: contents;
  }

  .toasts {
    position: fixed;
    right: $sp-6;
    bottom: $sp-6;
    display: grid;
    gap: $sp-2;
    justify-items: end;

    .toast {
      @include surface(3);
      display: flex;
      align-items: center;
      gap: $sp-3;
      padding: $sp-3 $sp-4;
      border-radius: $r-button;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);

      &.request {
        box-shadow: inset 0 0 0 1px $accent-fg, 0 8px 24px rgba(0, 0, 0, 0.25);
      }

      .tag {
        color: $text-2;
        font-size: $fs-small;
      }
    }
  }
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity $t-panel $ease,
    transform $t-panel $ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
