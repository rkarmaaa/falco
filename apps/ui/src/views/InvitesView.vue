<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import EmptyState from '@/components/EmptyState.vue';
import UiButton from '@/components/UiButton.vue';
import UiPage from '@/components/UiPage.vue';
import { loadInvites, revokeInvite, social } from '@/stores/social';
import { openModal } from '@/stores/ui';

const { t, d } = useI18n();
const active = computed(() => social.invites.filter((i) => i.active));
const history = computed(() => social.invites.filter((i) => !i.active));
const when = (ts: number) => d(new Date(ts * 1000), { dateStyle: 'medium', timeStyle: 'short' } as never);

onMounted(() => void loadInvites());
</script>

<template>
  <UiPage class="invites" :title="$t('invites.title')" narrow>
    <template #actions>
      <UiButton variant="primary" @click="openModal('share')">{{ $t('stream.share') }}</UiButton>
    </template>

    <EmptyState v-if="social.invites.length === 0" :title="$t('invites.emptyTitle')" :body="$t('invites.emptyBody')" />

    <template v-else>
      <h2 class="label">{{ $t('invites.active') }}</h2>
      <p v-if="active.length === 0" class="muted">{{ $t('invites.noneActive') }}</p>
      <ul class="list">
        <li v-for="i in active" :key="i.tokenHash" class="row">
          <div class="info">
            <span class="name">{{ $t(`role.${i.role}`) }}</span>
            <span class="meta">
              {{ t('invites.usesLeft', { n: i.usesLeft }, i.usesLeft) }} · {{ $t('invites.expires', { when: when(i.expiresAt) }) }}
              <template v-if="i.needsApproval"> · {{ $t('invites.approval') }}</template>
            </span>
          </div>
          <UiButton variant="danger" :size="32" @click="revokeInvite(i.tokenHash)">{{ $t('invites.revoke') }}</UiButton>
        </li>
      </ul>

      <h2 v-if="history.length" class="label">{{ $t('invites.history') }}</h2>
      <ul class="list">
        <li v-for="i in history" :key="i.tokenHash" class="row past">
          <div class="info">
            <span class="name">{{ $t(`role.${i.role}`) }}</span>
            <span class="meta">{{ $t('invites.created', { when: when(i.createdAt) }) }}</span>
          </div>
        </li>
      </ul>
    </template>
  </UiPage>
</template>

<style lang="scss" scoped>
.invites {
  .label {
    @include label;
    margin-top: $sp-3;
  }

  .muted {
    color: $text-3;
    font-size: $fs-small;
  }

  .list {
    display: grid;
    gap: $sp-2;
    margin: 0;
    padding: 0;
    list-style: none;

    .row {
      @include surface(2);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: $sp-3;
      padding: $sp-3 $sp-4;
      border-radius: $r-card;

      .info {
        display: grid;

        .name {
          font-weight: 600;
        }

        .meta {
          font-size: $fs-small;
          color: $text-2;
        }
      }

      &.past {
        opacity: 0.6;
      }
    }
  }
}
</style>
