<script setup lang="ts">
import { onMounted, ref, watchEffect } from 'vue';
import { usePreferredDark } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import TitleBar from '@/components/TitleBar.vue';
import SideBar, { type Section } from '@/components/SideBar.vue';
import CoreOffline from '@/components/CoreOffline.vue';
import FriendsView from '@/views/FriendsView.vue';
import HostView from '@/views/HostView.vue';
import InvitesView from '@/views/InvitesView.vue';
import SettingsView from '@/views/SettingsView.vue';
import { core, initCore } from '@/stores/core';
import { setLocale } from '@/i18n';

const { t } = useI18n();
const section = ref<Section>('friends');
const toast = ref<string | null>(null);
const prefersDark = usePreferredDark();

// Tema e lingua seguono config.toml; «system» segue il sistema operativo
watchEffect(() => {
  const theme = core.config?.general.theme ?? 'system';
  const dark = theme === 'dark' || (theme === 'system' && prefersDark.value);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  setLocale(core.config?.general.language ?? 'it');
});

function showToast(message: string): void {
  toast.value = message;
  setTimeout(() => (toast.value = null), 4000);
}

onMounted(() => void initCore());
</script>

<template>
  <div class="app">
    <TitleBar />
    <div class="body">
      <SideBar v-model="section" />
      <main class="content">
        <CoreOffline v-if="core.ready && !core.connected" />
        <div class="view">
          <FriendsView v-if="section === 'friends'" @invite="showToast(t('errors.notImplemented', { milestone: 'M3' }))" />
          <HostView v-else-if="section === 'host'" />
          <InvitesView v-else-if="section === 'invites'" />
          <SettingsView v-else />
        </div>
      </main>
    </div>
    <div class="toasts" aria-live="polite">
      <Transition name="toast">
        <div v-if="toast" class="toast">{{ toast }}</div>
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

  .toasts {
    position: fixed;
    right: $sp-6;
    bottom: $sp-6;

    .toast {
      @include surface(3);
      padding: $sp-3 $sp-4;
      border-radius: $r-button;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
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
