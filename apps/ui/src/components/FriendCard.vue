<script setup lang="ts">
// Card amico: stato, attività, azioni rapide. In gioco: bordo d'accento e alone (§12)
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Play, Send, Star } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import UiButton from './UiButton.vue';
import type { Friend } from '@/ipc';

const props = defineProps<{ friend: Friend; selected?: boolean; invited?: boolean }>();
const emit = defineEmits<{ open: []; join: []; invite: [] }>();
const { t } = useI18n();

const ring = computed(() =>
  props.friend.state === 'playing' ? 'playing' : props.friend.state === 'busy' ? 'busy' : props.friend.state === 'online' ? 'online' : 'offline',
);
const activity = computed(() => {
  const f = props.friend;
  if (f.state === 'playing') return t('friends.activity.playing', { n: f.slotsFree ?? 0 }, f.slotsFree ?? 0);
  if (f.remote) return t('friends.notConnected');
  if (f.state === 'offline' && f.lastSeen) return t('friends.activity.seen', { when: ago(f.lastSeen) });
  return t(`friends.state.${f.state}`);
});

function ago(ts: number): string {
  const min = Math.max(1, Math.round((Date.now() / 1000 - ts) / 60));
  return min < 60 ? t('time.minutes', { n: min }) : min < 1440 ? t('time.hours', { n: Math.round(min / 60) }) : t('time.days', { n: Math.round(min / 1440) });
}
</script>

<template>
  <li class="friend" :class="[friend.state, { selected }]">
    <button type="button" class="main" @click="emit('open')">
      <UiAvatar class="avatar" :name="friend.name" :src="friend.avatarUrl" :accent="friend.accent" :size="40" :status="ring" />
      <span class="info">
        <span class="name">
          {{ friend.name }}
          <Star v-if="friend.favorite" class="fav" :size="12" :stroke-width="1.75" :aria-label="$t('friends.favorite')" />
        </span>
        <span class="activity">{{ activity }}</span>
      </span>
    </button>
    <div class="actions">
      <UiButton v-if="friend.state === 'playing'" variant="primary" :size="32" @click="emit('join')">
        <Play :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('friends.join') }}
      </UiButton>
      <UiButton v-else variant="ghost" :size="32" :disabled="invited" @click="emit('invite')">
        <Check v-if="invited" :size="16" :stroke-width="1.75" aria-hidden="true" />
        <Send v-else :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ invited ? $t('friends.invited') : $t('friends.inviteOne') }}
      </UiButton>
    </div>
  </li>
</template>

<style lang="scss" scoped>
.friend {
  @include surface(2);
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: $sp-3;
  padding-right: $sp-4;
  border-radius: $r-card;
  transition: background $t-fast $ease;

  .main {
    @include reset-button;
    @include focus-ring;
    display: flex;
    align-items: center;
    gap: $sp-3;
    min-width: 0;
    padding: $sp-3 $sp-4;
    border-radius: $r-card;
    text-align: left;

    .info {
      display: grid;
      min-width: 0;

      .name {
        @include truncate;
        display: flex;
        align-items: center;
        gap: $sp-1;
        font-weight: 600;

        .fav {
          color: $busy;
        }
      }

      .activity {
        @include truncate;
        font-size: $fs-small;
        color: $text-2;
      }
    }
  }

  .actions {
    display: flex;
    gap: $sp-2;
  }

  &:hover,
  &.selected {
    background: $bg-3;
  }

  // In gioco: bordo d'accento
  &.playing {
    box-shadow: inset 0 0 0 1px $accent-fg;

    .activity {
      color: $accent-fg;
    }
  }

  &.offline .name {
    color: $text-2;
  }
}
</style>
