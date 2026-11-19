<script setup lang="ts">
// Barra delle azioni e dello stream (in basso). Da sinistra: tu, chi è collegato
// (se ospiti) o la sessione in corso (se sei ospite), poi le azioni; in fondo a destra
// «Schermo intero» quando guardi uno stream.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Activity,
  ChevronUp,
  Check,
  LogOut,
  Maximize2,
  MonitorCog,
  Power,
  Radio,
  Link2,
  SlidersHorizontal,
} from 'lucide-vue-next';
import UserCard from './UserCard.vue';
import GuestChip from './GuestChip.vue';
import DeviceIcons from './DeviceIcons.vue';
import UiAvatar from './UiAvatar.vue';
import UiButton from './UiButton.vue';
import ContextMenu from './ContextMenu.vue';
import StreamVolume from './StreamVolume.vue';
import { core, hostApi, leaveSession, reconnectSession, setConfig, setHostAvailable } from '@/stores/core';
import { social } from '@/stores/social';
import { attempt } from '@/stores/errors';
import { openModal } from '@/stores/ui';
import { setFullscreen, stage, toggleStats } from '@/stores/stage';
import { PROFILES, applyProfile, currentProfile } from '@/stores/share';
import type { HostProfile } from '@/ipc';

const emit = defineEmits<{ navigate: [string] }>();
const { t } = useI18n();
const profileMenu = ref<{ x: number; y: number } | null>(null);
const profileNote = ref<string | null>(null);

const hosting = computed(() => core.host.state === 'ready');
const starting = computed(() => core.host.state === 'starting');
const inSession = computed(() => core.session.phase !== 'idle');
const info = computed(() => core.session.info);
const hostFriend = computed(() => social.friends.find((f) => f.id === info.value?.friendId));
const myDevices = computed(() => {
  const p = info.value?.permissions;
  return { keyboard: !!p?.keyboard, mouse: !!p?.mouse, controller: (p?.gamepads ?? 0) > 0 };
});
const inputsPending = computed(() => core.host.message === 'inputs-pending');

const phaseText = computed(() => {
  const s = core.session;
  if (s.phase === 'starting') return t('session.knocking');
  if (s.phase === 'connecting') return t('session.connecting');
  if (!s.stats) return t('session.connected');
  return null;
});
const pingLevel = computed(() => {
  const ms = core.session.stats?.rttMs ?? 0;
  return ms < 30 ? 'good' : ms <= 60 ? 'fair' : 'bad';
});

// Qualità dello stream da ospite: a portata di mano nella barra, si ricollega da sola
const QUALITIES = ['light', 'good', 'max', 'custom'] as const;
type Quality = (typeof QUALITIES)[number];
const qualityMenu = ref<{ x: number; y: number } | null>(null);
const quality = computed(() => (core.config?.stream.quality ?? 'light') as Quality);

function openQuality(e: MouseEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  qualityMenu.value = { x: r.left, y: r.top - 8 };
}

async function pickQuality(q: Quality): Promise<void> {
  qualityMenu.value = null;
  if (q === quality.value) return;
  await attempt(t('errors.context.quality'), async () => {
    await setConfig('stream.quality', q);
    await reconnectSession();
  });
}

function openProfiles(e: MouseEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  profileMenu.value = { x: r.left, y: r.top - 8 };
}

async function pickProfile(p: HostProfile): Promise<void> {
  profileMenu.value = null;
  const r = await attempt(t('errors.context.profile'), () => applyProfile(p));
  profileNote.value = r === 'later' ? t('share.appliesLater') : r === 'now' ? t('share.appliedNow') : null;
  setTimeout(() => (profileNote.value = null), 5000);
}

const stop = () => attempt(t('errors.context.stopShare'), () => setHostAvailable(false));
const leave = () => attempt(t('errors.context.leave'), () => leaveSession());
const applyInputs = () => attempt(t('errors.context.applyInputs'), () => hostApi.applyInputs());
</script>

<template>
  <footer class="bar" :aria-label="t('bar.label')">
    <UserCard @profile="emit('navigate', 'settings')" />
    <span class="sep" aria-hidden="true" />

    <!-- Centro: la sessione che stai guardando, oppure chi è collegato da te -->
    <div class="middle">
      <div v-if="inSession" class="session" :class="core.session.phase">
        <UiAvatar :name="core.session.host ?? '?'" :src="hostFriend?.avatarUrl" :accent="hostFriend?.accent" :size="32" :status="core.session.phase === 'connected' ? 'playing' : 'busy'" />
        <div class="what">
          <span class="who">{{ t('session.watching', { name: core.session.host }) }}</span>
          <span class="how">
            <template v-if="phaseText">
              <span class="spinner" aria-hidden="true" />
              {{ phaseText }}
            </template>
            <template v-else-if="core.session.stats">
              <span class="ping" :class="pingLevel">{{ Math.round(core.session.stats.rttMs) }} ms</span>
              <template v-if="stage.stats">
                <span>{{ Math.round(core.session.stats.fps) }} fps</span>
                <span>{{ (core.session.stats.bitrateKbps / 1000).toFixed(1) }} Mbps</span>
                <span>{{ t(`session.path.${core.session.stats.path === 'relay' ? 'relay' : 'direct'}`) }}</span>
              </template>
            </template>
          </span>
        </div>
        <span class="mine" :title="t('session.yourDevices')">
          <DeviceIcons :allowed="myDevices" :size="14" />
        </span>
      </div>

      <template v-else-if="hosting">
        <ul v-if="social.guests.length" class="guests" :aria-label="t('guests.label')">
          <GuestChip v-for="g in social.guests" :key="g.friendId" :guest="g" />
        </ul>
        <p v-else class="hint">
          <Radio :size="16" :stroke-width="1.9" aria-hidden="true" />
          {{ t('bar.waiting') }}
        </p>
        <p v-if="inputsPending" class="pending" role="status">
          {{ t('guests.pending') }}
          <button type="button" class="link" @click="applyInputs">{{ t('guests.applyNow') }}</button>
        </p>
      </template>

      <p v-else-if="starting" class="hint"><span class="spinner" aria-hidden="true" />{{ t('bar.starting') }}</p>
      <p v-else class="hint">{{ t('bar.idle') }}</p>
      <p v-if="profileNote" class="note" role="status">{{ profileNote }}</p>
    </div>

    <!-- Azioni -->
    <div class="actions">
      <template v-if="inSession">
        <StreamVolume v-if="info?.pid" :pid="info.pid" />
        <UiButton variant="ghost" :size="40" :aria-haspopup="'menu'" @click="openQuality">
          <MonitorCog :size="17" :stroke-width="1.9" aria-hidden="true" />
          {{ t(`quality.short.${quality}`) }}
          <ChevronUp :size="14" :stroke-width="2" aria-hidden="true" />
        </UiButton>
        <UiButton variant="ghost" :size="40" :aria-pressed="stage.stats" :class="{ on: stage.stats }" @click="toggleStats">
          <Activity :size="17" :stroke-width="1.9" aria-hidden="true" />
          {{ t('session.stats') }}
        </UiButton>
        <UiButton variant="danger" :size="40" @click="leave">
          <LogOut :size="17" :stroke-width="1.9" aria-hidden="true" />
          {{ t('session.leave') }}
        </UiButton>
        <UiButton variant="primary" :size="40" class="fullscreen" :disabled="core.session.phase !== 'connected'" @click="setFullscreen(true)">
          <Maximize2 :size="17" :stroke-width="2" aria-hidden="true" />
          {{ t('session.fullscreen') }}
        </UiButton>
      </template>

      <template v-else>
        <UiButton v-if="hosting" variant="ghost" :size="40" :aria-haspopup="'menu'" @click="openProfiles">
          <SlidersHorizontal :size="17" :stroke-width="1.9" aria-hidden="true" />
          {{ t(`share.profiles.${currentProfile}.name`) }}
          <ChevronUp :size="14" :stroke-width="2" aria-hidden="true" />
        </UiButton>
        <!-- Interruttore: «Avvia» crea il link e condivide; acceso, un clic ferma -->
        <UiButton
          v-if="hosting"
          variant="ghost"
          :size="40"
          icon
          :title="t('stream.share')"
          :aria-label="t('stream.share')"
          @click="openModal('share')"
        >
          <Link2 :size="17" :stroke-width="1.9" aria-hidden="true" />
        </UiButton>
        <UiButton
          :variant="hosting || starting ? 'secondary' : 'primary'"
          :size="40"
          class="power"
          :class="{ on: hosting }"
          :aria-pressed="hosting"
          :title="hosting ? t('bar.stopHint') : undefined"
          :disabled="starting"
          @click="hosting ? stop() : openModal('share')"
        >
          <Power :size="17" :stroke-width="2.2" aria-hidden="true" />
          {{ hosting ? t('bar.on') : starting ? t('host.state.starting') : t('bar.start') }}
        </UiButton>
      </template>
    </div>

    <ContextMenu v-if="qualityMenu" :x="qualityMenu.x" :y="qualityMenu.y" :label="t('quality.title')" @close="qualityMenu = null">
      <div class="head">
        <strong>{{ t('quality.title') }}</strong>
        <span>{{ t('quality.hint') }}</span>
      </div>
      <hr />
      <button
        v-for="q in QUALITIES"
        :key="q"
        type="button"
        class="item"
        role="menuitemradio"
        :aria-checked="quality === q"
        @click="pickQuality(q)"
      >
        <span class="grow">
          {{ t(`quality.short.${q}`) }}
          <span class="hint">· {{ t(`quality.hints.${q}`) }}</span>
        </span>
        <Check v-if="quality === q" class="check" :size="16" :stroke-width="2.4" aria-hidden="true" />
      </button>
    </ContextMenu>

    <ContextMenu v-if="profileMenu" :x="profileMenu.x" :y="profileMenu.y" :label="t('share.title')" @close="profileMenu = null">
      <div class="head">
        <strong>{{ t('share.title') }}</strong>
        <span>{{ t('share.menuHint') }}</span>
      </div>
      <hr />
      <button
        v-for="p in PROFILES"
        :key="p"
        type="button"
        class="item"
        role="menuitemradio"
        :aria-checked="currentProfile === p"
        @click="pickProfile(p)"
      >
        <span class="grow">
          {{ t(`share.profiles.${p}.name`) }}
          <span class="hint">· {{ t(`share.profiles.${p}.short`) }}</span>
        </span>
        <Check v-if="currentProfile === p" class="check" :size="16" :stroke-width="2.4" aria-hidden="true" />
      </button>
      <hr />
      <button type="button" class="item" role="menuitem" @click="(profileMenu = null), emit('navigate', 'host')">
        <SlidersHorizontal :size="16" :stroke-width="1.9" aria-hidden="true" />
        <span class="grow">{{ t('share.more') }}</span>
      </button>
    </ContextMenu>
  </footer>
</template>

<style lang="scss" scoped>
.bar {
  position: relative;
  display: flex;
  align-items: center;
  gap: $sp-3;
  height: $actionbar-h;
  padding: 0 $sp-4 0 $sp-3;
  border-top: 1px solid $line;
  background: linear-gradient(180deg, rgba(14, 16, 28, 0.92), rgba(9, 10, 18, 0.98));
  backdrop-filter: blur(16px);

  .sep {
    width: 1px;
    height: 32px;
    background: $line;
  }

  .middle {
    position: relative;
    display: flex;
    flex: 1;
    align-items: center;
    gap: $sp-3;
    min-width: 0;
    overflow: hidden;

    .hint {
      display: flex;
      align-items: center;
      gap: $sp-2;
      color: $text-3;
      font-size: $fs-small;
    }

    .note,
    .pending {
      padding: 6px $sp-3;
      border-radius: $r-pill;
      background: $accent-soft;
      color: $accent-fg;
      font-size: 12px;
      white-space: nowrap;
    }

    .pending {
      background: rgba(245, 184, 61, 0.12);
      color: $busy;

      .link {
        @include reset-button;
        @include focus-ring;
        margin-left: $sp-1;
        font-weight: 650;
        text-decoration: underline;
        text-underline-offset: 2px;
      }
    }
  }

  .guests {
    display: flex;
    gap: $sp-2;
    margin: 0;
    padding: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .session {
    display: flex;
    align-items: center;
    gap: $sp-3;
    min-width: 0;
    padding: 0 $sp-3 0 $sp-2;
    height: 52px;
    border: 1px solid $line;
    border-radius: $r-card;
    background: $bg-2;

    &.connected {
      border-color: rgba(70, 224, 169, 0.28);
      box-shadow: 0 0 24px rgba(70, 224, 169, 0.08);
    }

    .what {
      display: grid;
      min-width: 0;

      .who {
        @include truncate;
        font-size: $fs-small;
        font-weight: 650;
      }

      .how {
        display: flex;
        align-items: center;
        gap: $sp-3;
        font-size: 12px;
        color: $text-3;
        font-variant-numeric: tabular-nums;
        white-space: nowrap;

        .ping {
          font-weight: 650;

          &.good {
            color: $ok;
          }

          &.fair {
            color: $busy;
          }

          &.bad {
            color: $danger;
          }
        }
      }
    }

    .mine {
      padding-left: $sp-3;
      border-left: 1px solid $line;
    }
  }

  .actions {
    display: flex;
    align-items: center;
    gap: $sp-2;

    .on {
      background: $accent-soft;
      color: $accent-fg;
    }

    .share {
      box-shadow: 0 0 0 1px rgba(157, 169, 255, 0.25), 0 8px 24px rgba(68, 86, 200, 0.35);
    }
  }
}

.spinner {
  width: 12px;
  height: 12px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: $r-pill;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.power.on {
  border-color: $live;

  :deep(svg) {
    color: $live;
    filter: drop-shadow(0 0 6px $live-glow);
  }
}
</style>
