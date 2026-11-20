<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import EmptyState from '@/components/EmptyState.vue';
import UiButton from '@/components/UiButton.vue';
import UiPage from '@/components/UiPage.vue';
import { loadInvites, revokeInvite, social } from '@/stores/social';
import { openModal } from '@/stores/ui';
import { History, Ticket } from 'lucide-vue-next';

const { t, d } = useI18n();
const active = computed(() => social.invites.filter((i) => i.active));
// Storico solo su richiesta e solo i più recenti
const history = computed(() => social.invites.filter((i) => !i.active).slice(0, 10));
const showHistory = ref(false);
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
      <p v-if="active.length === 0" class="muted">{{ $t('invites.noneActive') }}</p>
      <ul v-else class="list">
        <li v-for="i in active" :key="i.tokenHash" class="card">
          <span class="icon"><Ticket :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
          <div class="info">
            <span class="name">{{ $t(`role.${i.role}`) }}</span>
            <span class="meta">
              {{ t('invites.usesLeft', { n: i.usesLeft }, i.usesLeft) }} · {{ $t('invites.expires', { when: when(i.expiresAt) }) }}
              <template v-if="i.needsApproval"> · {{ $t('invites.approval') }}</template>
            </span>
          </div>
          <UiButton variant="ghost" :size="32" @click="revokeInvite(i.tokenHash)">{{ $t('invites.revoke') }}</UiButton>
        </li>
      </ul>

      <UiButton v-if="history.length" variant="ghost" :size="32" class="toggle" @click="showHistory = !showHistory">
        <History :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ showHistory ? $t('invites.hideHistory') : $t('invites.showHistory', { n: history.length }) }}
      </UiButton>
      <ul v-if="showHistory" class="list past">
        <li v-for="i in history" :key="i.tokenHash" class="card">
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
  .muted {
    @include message;
  }

  .toggle {
    justify-self: start;
  }

  .list {
    display: grid;
    gap: $sp-2;
    margin: 0;
    padding: 0;
    list-style: none;

    .card {
      @include surface(2);
      display: flex;
      align-items: center;
      gap: $sp-3;
      padding: $sp-3 $sp-4;
      border-radius: $r-card;

      .icon {
        display: grid;
        place-items: center;
        width: 36px;
        height: 36px;
        border-radius: $r-button;
        background: $accent-soft;
        color: $accent-fg;
      }

      .info {
        display: grid;
        flex: 1;
        min-width: 0;

        .name {
          font-weight: $fw-strong;
        }

        .meta {
          @include message($text-3);
        }
      }
    }

    &.past .card {
      opacity: 0.7;
    }
  }
}
</style>
