<script setup lang="ts">
// Schermata principale. In M0 solo lo stato vuoto: gli amici arrivano con M2.
import { UserPlus } from 'lucide-vue-next';
import EmptyState from '@/components/EmptyState.vue';
import UiButton from '@/components/UiButton.vue';

const emit = defineEmits<{ invite: [] }>();
</script>

<template>
  <section class="friends">
    <header class="head">
      <h1 class="title">{{ $t('friends.title') }}</h1>
      <UiButton variant="primary" @click="emit('invite')">
        <UserPlus :size="18" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('friends.invite') }}
      </UiButton>
    </header>
    <EmptyState :title="$t('friends.emptyTitle')" :body="$t('friends.emptyBody')">
      <UiButton variant="secondary" :size="48" @click="emit('invite')">
        {{ $t('friends.emptyAction') }}
      </UiButton>
    </EmptyState>
  </section>
</template>

<style lang="scss" scoped>
.friends {
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: $sp-4;

    .title {
      font-size: $fs-h1;
      line-height: 30px;
      font-weight: 650;
    }
  }
}
</style>
