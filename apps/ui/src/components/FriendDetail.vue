<script setup lang="ts">
// Pannello destro (cassetto sotto sm): avatar 96, tag, dispositivi, ruolo, azioni
import { ref } from 'vue';
import { Ban, Monitor, Play, Star, Trash2, X } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import UiButton from './UiButton.vue';
import PermissionEditor from './PermissionEditor.vue';
import { rpc } from '@/ipc';
import { friendApi } from '@/stores/social';
import type { Friend } from '@/ipc';

const props = defineProps<{ friend: Friend }>();
const emit = defineEmits<{ close: []; join: [] }>();
const confirm = ref<'remove' | 'block' | null>(null);

async function act(): Promise<void> {
  if (confirm.value === 'remove') await friendApi.remove(props.friend.id);
  if (confirm.value === 'block') await friendApi.block(props.friend.id);
  confirm.value = null;
  emit('close');
}
</script>

<template>
  <aside class="detail" :aria-label="friend.name">
    <button type="button" class="close" :aria-label="$t('app.close')" @click="emit('close')">
      <X :size="18" :stroke-width="1.75" />
    </button>
    <UiAvatar :name="friend.name" :src="friend.avatarUrl" :accent="friend.accent" :size="96" :status="friend.state === 'playing' ? 'playing' : null" />
    <h2 class="name">{{ friend.name }}</h2>
    <p class="tag">{{ friend.tag }}</p>
    <span v-if="friend.role" class="chip" :title="$t(`role.hint.${friend.role}`)">{{ $t(`role.${friend.role}`) }}</span>

    <UiButton v-if="friend.canJoin" variant="primary" class="primary" @click="emit('join')">
      <Play :size="18" :stroke-width="1.75" aria-hidden="true" />
      {{ $t('friends.join') }}
    </UiButton>
    <p v-else class="muted">{{ $t(`friends.state.${friend.state}`) }}</p>

    <UiButton v-if="!friend.canJoin && friend.state !== 'offline'" variant="secondary" :size="32" @click="rpc('friend.notify', { id: friend.id })">
      {{ $t('friends.invitePlay') }}
    </UiButton>

    <PermissionEditor :friend-id="friend.id" />

    <h3 class="group">{{ $t('friends.devices') }}</h3>
    <ul class="devices">
      <li v-for="d in friend.devices" :key="d.tsNodeId" class="device">
        <Monitor :size="16" :stroke-width="1.75" aria-hidden="true" />
        <span class="dname">{{ d.hostname || d.tsIp }}</span>
        <span class="meta">{{ d.os }}</span>
      </li>
    </ul>

    <div class="tools">
      <UiButton variant="ghost" :size="32" @click="friendApi.favorite(friend.id, !friend.favorite)">
        <Star :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ friend.favorite ? $t('friends.unfavorite') : $t('friends.makeFavorite') }}
      </UiButton>
      <UiButton variant="ghost" :size="32" @click="confirm = 'remove'">
        <Trash2 :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('friends.remove') }}
      </UiButton>
      <UiButton variant="ghost" :size="32" @click="confirm = 'block'">
        <Ban :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('friends.block') }}
      </UiButton>
    </div>
    <div v-if="confirm" class="confirm" role="alert">
      <p>{{ $t(`friends.confirm.${confirm}`, { name: friend.name }) }}</p>
      <div class="row">
        <UiButton variant="danger" :size="32" @click="act">{{ $t(`friends.${confirm}`) }}</UiButton>
        <UiButton variant="ghost" :size="32" @click="confirm = null">{{ $t('friends.cancel') }}</UiButton>
      </div>
    </div>
  </aside>
</template>

<style lang="scss" scoped>
.detail {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: $sp-2;
  width: $detail-w;
  flex: 0 0 $detail-w;
  padding: $sp-7 $sp-5;
  overflow-y: auto;
  background: $bg-1;
  border-left: 1px solid $line;

  // Sotto sm il pannello diventa un cassetto sovrapposto
  @include mq(sm) {
    position: fixed;
    top: $titlebar-h;
    right: 0;
    bottom: 0;
    z-index: 10;
    box-shadow: -16px 0 40px rgba(0, 0, 0, 0.3);
  }

  .close {
    @include reset-button;
    @include focus-ring;
    position: absolute;
    top: $sp-3;
    right: $sp-3;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: $r-button;
    color: $text-2;

    &:hover {
      background: $bg-3;
    }
  }

  .name {
    margin-top: $sp-2;
    font-size: $fs-h2;
    line-height: 24px;
    font-weight: 600;
  }

  .tag {
    color: $text-2;
    font-size: $fs-small;
    font-variant-numeric: tabular-nums;
  }

  .chip {
    padding: 2px $sp-3;
    border: 1px solid $line;
    border-radius: $r-pill;
    font-size: $fs-small;
    color: $text-2;
  }

  .primary {
    align-self: stretch;
    margin-top: $sp-3;
  }

  .muted {
    margin-top: $sp-3;
    color: $text-3;
    font-size: $fs-small;
  }

  .group {
    @include label;
    align-self: flex-start;
    margin-top: $sp-5;
  }

  .devices {
    align-self: stretch;
    display: grid;
    gap: $sp-1;
    margin: 0;
    padding: 0;
    list-style: none;

    .device {
      display: flex;
      align-items: center;
      gap: $sp-2;
      font-size: $fs-small;

      .dname {
        @include truncate;
        flex: 1;
      }

      .meta {
        color: $text-3;
      }
    }
  }

  .tools {
    align-self: stretch;
    display: grid;
    margin-top: auto;
    padding-top: $sp-5;
    justify-items: start;
  }

  .confirm {
    align-self: stretch;
    display: grid;
    gap: $sp-2;
    padding: $sp-3;
    border: 1px solid $danger;
    border-radius: $r-card;
    font-size: $fs-small;

    .row {
      display: flex;
      gap: $sp-2;
    }
  }
}
</style>
