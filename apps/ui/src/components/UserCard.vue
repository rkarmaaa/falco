<script setup lang="ts">
// Card utente in basso a sinistra: foto, nome e, sotto, l'ID oppure lo stato
// («In condivisione», «Offline»). Clic: stato e profilo.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Copy, Pencil } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import ContextMenu from './ContextMenu.vue';
import { core, setStatus } from '@/stores/core';
import { social } from '@/stores/social';
import { attempt } from '@/stores/errors';
import { copyText } from '@/utils/clipboard';
import type { Status } from '@/ipc';

const emit = defineEmits<{ profile: [] }>();
const { t } = useI18n();
const menu = ref<{ x: number; y: number } | null>(null);
const copied = ref(false);

const sharing = computed(() => core.host.state === 'ready');
const offline = computed(() => !core.connected || core.snapshot?.status === 'invisible');
const line = computed(() => {
  if (sharing.value) return { kind: 'live', text: t('me.sharing') };
  if (offline.value) return { kind: 'off', text: t('me.offline') };
  if (core.snapshot?.status === 'busy') return { kind: 'busy', text: t('status.busy') };
  return { kind: 'id', text: social.profile.tag ?? '' };
});
const ring = computed(() => (sharing.value ? 'playing' : offline.value ? 'offline' : core.snapshot?.status === 'busy' ? 'busy' : 'online'));

function open(e: MouseEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  menu.value = { x: r.left, y: r.top - 8 };
}

async function copyId(): Promise<void> {
  copied.value = await copyText(social.profile.tag ?? '');
  setTimeout(() => (copied.value = false), 1600);
}

function pick(s: Status): void {
  menu.value = null;
  void attempt(t('errors.context.status'), () => setStatus(s));
}
</script>

<template>
  <div class="me">
    <button type="button" class="card" :aria-haspopup="'menu'" :aria-expanded="!!menu" @click="open" @contextmenu.prevent="open">
      <UiAvatar :name="social.profile.name ?? '?'" :src="social.profile.avatarUrl" :accent="social.profile.accent" :size="40" :status="ring" />
      <span class="info">
        <span class="name">{{ social.profile.name }}</span>
        <span class="line" :class="line.kind">
          <span v-if="line.kind !== 'id'" class="dot" aria-hidden="true" />
          {{ line.text }}
        </span>
      </span>
    </button>
    <button v-if="line.kind === 'id'" type="button" class="copy" :title="t('me.copyId')" :aria-label="t('me.copyId')" @click="copyId">
      <Check v-if="copied" :size="14" :stroke-width="2.4" aria-hidden="true" />
      <Copy v-else :size="14" :stroke-width="1.9" aria-hidden="true" />
    </button>

    <ContextMenu v-if="menu" :x="menu.x" :y="menu.y" :label="t('nav.me')" @close="menu = null">
      <div class="head">
        <strong>{{ social.profile.name }}</strong>
        <span>{{ social.profile.tag }}</span>
      </div>
      <hr />
      <button
        v-for="s in ['online', 'busy', 'invisible'] as Status[]"
        :key="s"
        type="button"
        class="item"
        role="menuitemradio"
        :aria-checked="core.snapshot?.status === s"
        @click="pick(s)"
      >
        <span class="status-dot" :class="s" aria-hidden="true" />
        <span class="grow">{{ t(`status.${s}`) }}</span>
        <Check v-if="core.snapshot?.status === s" class="check" :size="16" :stroke-width="2.4" aria-hidden="true" />
      </button>
      <hr />
      <button type="button" class="item" role="menuitem" @click="(menu = null), emit('profile')">
        <Pencil :size="16" :stroke-width="1.9" aria-hidden="true" />
        <span class="grow">{{ t('me.editProfile') }}</span>
      </button>
    </ContextMenu>
  </div>
</template>

<style lang="scss" scoped>
.me {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;

  .card {
    @include reset-button;
    @include focus-ring;
    display: flex;
    align-items: center;
    gap: $sp-3;
    min-width: 0;
    height: 52px;
    padding: 0 $sp-3 0 $sp-2;
    border-radius: $r-card;
    transition: background $t-fast $ease;

    &:hover,
    &[aria-expanded='true'] {
      background: $bg-3;
    }

    .info {
      display: grid;
      min-width: 0;
      text-align: left;

      .name {
        @include truncate;
        max-width: 160px;
        font-weight: 650;
      }

      .line {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        line-height: 16px;
        color: $text-3;
        font-variant-numeric: tabular-nums;

        .dot {
          width: 7px;
          height: 7px;
          border-radius: $r-pill;
          background: currentColor;
        }

        &.live {
          color: $live;

          .dot {
            box-shadow: 0 0 8px $live-glow;
            animation: breathe 2.4s $ease infinite;
          }
        }

        &.busy {
          color: $busy;
        }
      }
    }
  }

  .copy {
    @include reset-button;
    @include focus-ring;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: $r-field;
    color: $text-3;

    &:hover {
      background: $bg-3;
      color: $text-1;
    }
  }
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: $r-pill;
  background: $ok;

  &.busy {
    background: $busy;
  }

  &.invisible {
    background: transparent;
    box-shadow: inset 0 0 0 2px $text-3;
  }
}

@keyframes breathe {
  50% {
    opacity: 0.45;
  }
}
</style>
