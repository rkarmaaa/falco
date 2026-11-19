<script setup lang="ts">
// Barra in alto, compatta e trascinabile: marchio, sezioni, scheda della sessione in corso,
// stato della rete e controlli della finestra (su macOS restano i semafori di sistema).
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Minus, Square, X } from 'lucide-vue-next';
import logo from '@/assets/falco-256.png';
import { core } from '@/stores/core';
import { isTauri } from '@/ipc';

export type Section = 'stream' | 'friends' | 'host' | 'invites' | 'settings';

const model = defineModel<Section>({ required: true });
const props = defineProps<{ requests?: number; friendRequests?: number }>();
const { t } = useI18n();
const isMac = navigator.userAgent.includes('Mac');
const showControls = isTauri() && !isMac;

const tabs = computed(() =>
  (['friends', 'host', 'invites', 'settings'] as const).map((id) => ({
    id,
    badge: (id === 'invites' ? props.requests : id === 'friends' ? props.friendRequests : 0) || null,
    live: id === 'host' && core.host.state === 'ready',
  })),
);
const inSession = computed(() => core.session.phase !== 'idle');
const net = computed(() => {
  if (!core.connected) return { level: 'bad', text: t('top.coreOffline') };
  const ts = core.snapshot?.network.tailscale;
  if (ts === 'running') return { level: 'ok', text: t('top.netReady') };
  if (ts === 'needsLogin') return { level: 'warn', text: t('top.netLogin') };
  return { level: 'warn', text: t('top.netSetup') };
});

async function win() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  return getCurrentWindow();
}
</script>

<template>
  <header class="top" :class="{ mac: isMac, quiet: inSession }" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <img :src="logo" alt="" width="22" height="22" />
      <span data-tauri-drag-region>Falco</span>
      <span class="beta" :title="t('app.betaHint')">BETA</span>
    </div>

    <nav class="tabs" :aria-label="t('app.name')">
      <button
        v-if="inSession"
        type="button"
        class="tab session"
        :class="{ active: model === 'stream' }"
        :aria-current="model === 'stream' ? 'page' : undefined"
        @click="model = 'stream'"
      >
        <span class="pulse" aria-hidden="true" />
        {{ core.session.host }}
      </button>
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        class="tab"
        :class="{ active: model === tab.id }"
        :aria-current="model === tab.id ? 'page' : undefined"
        @click="model = tab.id"
      >
        <span v-if="tab.live" class="live" :title="t('me.sharing')" aria-hidden="true" />
        {{ t(`nav.${tab.id}`) }}
        <span v-if="tab.badge" class="badge">{{ tab.badge }}</span>
      </button>
    </nav>

    <div class="right" data-tauri-drag-region>
      <span class="net" :class="net.level" :title="net.text" role="status">
        <span class="dot" aria-hidden="true" />
        {{ net.text }}
      </span>
      <div v-if="showControls" class="controls">
        <button type="button" class="control" :aria-label="t('app.minimize')" @click="win().then((w) => w.minimize())">
          <Minus :size="14" :stroke-width="1.75" />
        </button>
        <button type="button" class="control" :aria-label="t('app.maximize')" @click="win().then((w) => w.toggleMaximize())">
          <Square :size="11" :stroke-width="1.75" />
        </button>
        <button type="button" class="control close" :aria-label="t('app.close')" @click="win().then((w) => w.close())">
          <X :size="15" :stroke-width="1.75" />
        </button>
      </div>
    </div>
  </header>
</template>

<style lang="scss" scoped>
.top {
  display: flex;
  align-items: center;
  gap: $sp-5;
  height: $topbar-h;
  padding-left: $sp-4;
  border-bottom: 1px solid $line;
  background: rgba(9, 10, 18, 0.7);
  backdrop-filter: blur(16px);
  transition:
    background $t-panel $ease,
    opacity $t-panel $ease;

  // Durante uno stream la barra si fa da parte finché non ci passi sopra
  &.quiet:not(:hover, :focus-within) {
    background: rgba(9, 10, 18, 0.35);
    opacity: 0.55;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: $sp-2;
    font-weight: 700;
    letter-spacing: -0.01em;

    img {
      border-radius: 6px;
    }

    .beta {
      padding: 1px $sp-2;
      border-radius: $r-pill;
      background: $busy-soft;
      color: $busy;
      font-size: $fs-label;
      line-height: 16px;
      font-weight: 700;
      letter-spacing: 0.08em;
    }
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 3px;
    border: 1px solid $line;
    border-radius: $r-button;
    background: rgba(255, 255, 255, 0.02);

    .tab {
      @include reset-button;
      @include focus-ring;
      display: flex;
      align-items: center;
      gap: 6px;
      height: 30px;
      padding: 0 $sp-3;
      border-radius: 7px;
      color: $text-2;
      font-size: $fs-small;
      font-weight: 550;
      transition:
        background $t-fast $ease,
        color $t-fast $ease;

      &:hover {
        color: $text-1;
        background: rgba(255, 255, 255, 0.04);
      }

      &.active {
        color: $text-1;
        background: $bg-4;
        box-shadow: $shadow-1, 0 1px 0 rgba(255, 255, 255, 0.06) inset;
      }

      &.session {
        color: $live;

        &.active {
          background: $live-soft;
        }

        .pulse {
          width: 7px;
          height: 7px;
          border-radius: $r-pill;
          background: $live;
          box-shadow: 0 0 0 0 $live-glow;
          animation: pulse 1.8s $ease infinite;
        }
      }

      .live {
        width: 6px;
        height: 6px;
        border-radius: $r-pill;
        background: $live;
        box-shadow: 0 0 8px $live-glow;
      }

      .badge {
        display: grid;
        place-items: center;
        min-width: 18px;
        height: 18px;
        padding: 0 5px;
        border-radius: $r-pill;
        background: $accent-500;
        color: $on-accent;
        font-size: 11px;
        font-weight: 700;
      }
    }
  }

  .right {
    display: flex;
    flex: 1;
    align-items: center;
    justify-content: flex-end;
    gap: $sp-3;
    height: 100%;

    .net {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 12px;
      color: $text-3;
      white-space: nowrap;

      .dot {
        width: 7px;
        height: 7px;
        border-radius: $r-pill;
        background: $ok;
      }

      &.warn .dot {
        background: $busy;
      }

      &.bad .dot {
        background: $danger;
      }
    }
  }

  .controls {
    display: flex;
    height: 100%;

    .control {
      @include reset-button;
      @include focus-ring;
      display: grid;
      place-items: center;
      width: 46px;
      height: 100%;
      color: $text-2;
      transition: background $t-fast $ease;

      &:hover {
        background: $bg-3;
        color: $text-1;
      }

      &.close:hover {
        background: $danger;
        color: $on-accent;
      }
    }
  }

  &.mac {
    padding-left: 84px;
  }

  @include mq(sm) {
    .net {
      display: none !important;
    }
  }
}

@keyframes pulse {
  70% {
    box-shadow: 0 0 0 6px transparent;
  }
}
</style>
