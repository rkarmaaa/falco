<script setup lang="ts">
// Richieste d'amicizia: ricevute (Accetta/Rifiuta) o inviate (Annulla)
import { useI18n } from 'vue-i18n';
import UiAvatar from './UiAvatar.vue';
import UiButton from './UiButton.vue';
import { declineFriend } from '@/stores/social';
import { acceptFriendRequest } from '@/stores/ui';
import { attempt } from '@/stores/errors';
import type { FriendRequest } from '@/ipc';

defineProps<{ requests: readonly FriendRequest[]; outgoing?: boolean }>();
const { t } = useI18n();
const code = (c: string) => (c.length === 8 ? `${c.slice(0, 4)}-${c.slice(4)}` : c);
</script>

<template>
  <ul class="requests">
    <li v-for="r in requests" :key="r.id" class="row">
      <UiAvatar :name="r.user.name" :size="40" />
      <div class="who">
        <span class="name">{{ r.user.name }}</span>
        <span class="meta">#{{ code(r.user.code) }} · {{ outgoing ? t('addFriend.waiting') : t('addFriend.wants') }}</span>
      </div>
      <div class="acts">
        <template v-if="outgoing">
          <UiButton variant="ghost" :size="32" @click="attempt(t('errors.context.request'), () => declineFriend(r.id))">{{ t('addFriend.cancel') }}</UiButton>
        </template>
        <template v-else>
          <UiButton variant="ghost" :size="32" @click="attempt(t('errors.context.request'), () => declineFriend(r.id))">{{ t('requests.decline') }}</UiButton>
          <UiButton variant="primary" :size="32" @click="acceptFriendRequest(r.id, r.user.name)">{{ t('requests.accept') }}</UiButton>
        </template>
      </div>
    </li>
  </ul>
</template>

<style lang="scss" scoped>
.requests {
  display: grid;
  gap: $sp-2;
  margin: 0;
  padding: 0;
  list-style: none;

  .row {
    @include surface(2);
    display: flex;
    align-items: center;
    gap: $sp-3;
    padding: $sp-3 $sp-4;
    border-radius: $r-card;

    .who {
      display: grid;
      flex: 1;
      min-width: 0;

      .name {
        @include truncate;
        font-weight: $fw-strong;
      }

      .meta {
        @include message($text-3);
      }
    }

    .acts {
      display: flex;
      gap: $sp-1;
    }
  }
}
</style>
