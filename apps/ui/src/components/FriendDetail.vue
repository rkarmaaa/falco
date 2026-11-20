<script setup lang="ts">
// Scheda dell'amico in un popup: foto e dati a sinistra, poi azioni, permessi e dispositivi
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Ban, Monitor, Play, Send, Star, Trash2 } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiAvatar from './UiAvatar.vue';
import UiButton from './UiButton.vue';
import PermissionEditor from './PermissionEditor.vue';
import { rpc } from '@/ipc';
import { friendApi } from '@/stores/social';
import type { Friend } from '@/ipc';

const props = defineProps<{ friend: Friend }>();
const emit = defineEmits<{ close: []; join: [] }>();
const { t } = useI18n();
const confirm = ref<'remove' | 'block' | null>(null);
const ring = computed(() =>
  props.friend.state === 'playing' ? 'playing' : props.friend.state === 'busy' ? 'busy' : props.friend.state === 'online' ? 'online' : 'offline',
);

async function act(): Promise<void> {
  if (confirm.value === 'remove') await friendApi.remove(props.friend.id);
  if (confirm.value === 'block') await friendApi.block(props.friend.id);
  confirm.value = null;
  emit('close');
}
</script>

<template>
  <UiModal :title="t('friends.detailTitle')" class="friend-modal" @close="emit('close')">
    <header class="who">
      <UiAvatar :name="friend.name" :src="friend.avatarUrl" :accent="friend.accent" :size="96" :status="ring" />
      <div class="info">
        <h2 class="name">{{ friend.name }}</h2>
        <span class="tag">{{ friend.tag }}</span>
        <div class="chips">
          <span class="chip state" :class="friend.state">{{ t(`friends.state.${friend.state}`) }}</span>
          <span v-if="friend.role" class="chip" :title="t(`role.hint.${friend.role}`)">{{ t(`role.${friend.role}`) }}</span>
          <span v-if="friend.favorite" class="chip"><Star :size="12" :stroke-width="2" aria-hidden="true" /> {{ t('friends.favorite') }}</span>
        </div>
      </div>
    </header>

    <div class="actions">
      <UiButton v-if="friend.canJoin" variant="primary" @click="emit('join')">
        <Play :size="18" :stroke-width="1.75" aria-hidden="true" />
        {{ t('friends.join') }}
      </UiButton>
      <UiButton v-if="!friend.canJoin && friend.state !== 'offline'" variant="secondary" @click="rpc('friend.notify', { id: friend.id })">
        <Send :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ t('friends.invitePlay') }}
      </UiButton>
    </div>

    <PermissionEditor :friend-id="friend.id" />

    <section v-if="friend.devices.length" class="devices">
      <h3 class="label">{{ t('friends.devices') }}</h3>
      <ul>
        <li v-for="d in friend.devices" :key="d.tsNodeId">
          <Monitor :size="16" :stroke-width="1.75" aria-hidden="true" />
          <span class="dname">{{ d.hostname || d.tsIp }}</span>
          <span class="meta">{{ d.os }}</span>
        </li>
      </ul>
    </section>

    <div v-if="confirm" class="confirm" role="alert">
      <p>{{ t(`friends.confirm.${confirm}`, { name: friend.name }) }}</p>
      <div class="row">
        <UiButton variant="danger" :size="32" @click="act">{{ t(`friends.${confirm}`) }}</UiButton>
        <UiButton variant="ghost" :size="32" @click="confirm = null">{{ t('friends.cancel') }}</UiButton>
      </div>
    </div>

    <template #actions>
      <UiButton variant="ghost" :size="32" @click="friendApi.favorite(friend.id, !friend.favorite)">
        <Star :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ friend.favorite ? t('friends.unfavorite') : t('friends.makeFavorite') }}
      </UiButton>
      <UiButton variant="ghost" :size="32" @click="confirm = 'remove'">
        <Trash2 :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ t('friends.remove') }}
      </UiButton>
      <UiButton variant="ghost" :size="32" @click="confirm = 'block'">
        <Ban :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ t('friends.block') }}
      </UiButton>
    </template>
  </UiModal>
</template>

<style lang="scss" scoped>
.who {
  display: flex;
  align-items: center;
  gap: $sp-5;

  .info {
    display: grid;
    gap: $sp-1;
    min-width: 0;

    .name {
      @include title-h1;
      @include truncate;
    }

    .tag {
      @include message($text-3);
      font-variant-numeric: tabular-nums;
    }

    .chips {
      display: flex;
      flex-wrap: wrap;
      gap: $sp-1;
      margin-top: $sp-1;
    }
  }
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px $sp-2;
  border-radius: $r-pill;
  background: $bg-3;
  color: $text-2;
  font-size: $fs-label;
  font-weight: $fw-strong;

  &.state.online,
  &.state.playing {
    background: $live-soft;
    color: $live;
  }

  &.state.busy {
    background: $busy-soft;
    color: $busy;
  }
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: $sp-2;

  &:empty {
    display: none;
  }
}

.devices {
  display: grid;
  gap: $sp-2;

  .label {
    @include label;
  }

  ul {
    display: grid;
    gap: $sp-1;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: $sp-2;
    color: $text-2;

    .dname {
      @include truncate;
      color: $text-1;
    }

    .meta {
      @include message($text-3);
      margin-left: auto;
    }
  }
}

.confirm {
  display: grid;
  gap: $sp-2;
  padding: $sp-3 $sp-4;
  border-radius: $r-card;
  background: $danger-soft;

  p {
    @include message($text-1);
  }

  .row {
    display: flex;
    gap: $sp-2;
  }
}
</style>
