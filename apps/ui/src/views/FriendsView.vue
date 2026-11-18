<script setup lang="ts">
// Schermata principale (§13): ricerca, «In gioco ora», «Online», «Offline» chiuso di default
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Ticket, UserPlus } from 'lucide-vue-next';
import EmptyState from '@/components/EmptyState.vue';
import UiButton from '@/components/UiButton.vue';
import FriendCard from '@/components/FriendCard.vue';
import FriendDetail from '@/components/FriendDetail.vue';
import { social } from '@/stores/social';
import { joinFriend } from '@/stores/core';
import { isRpcError, type Friend } from '@/ipc';

const emit = defineEmits<{ invite: []; accept: []; error: [string] }>();
const { t } = useI18n();
const query = ref('');
const selectedId = ref<string | null>(null);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return social.friends.filter((f) => !q || f.name.toLowerCase().includes(q) || f.tag.toLowerCase().includes(q));
});
const groups = computed(() => ({
  playing: filtered.value.filter((f) => f.state === 'playing'),
  online: filtered.value.filter((f) => ['online', 'busy', 'pending'].includes(f.state)),
  offline: filtered.value.filter((f) => f.state === 'offline'),
}));
const selected = computed(() => social.friends.find((f) => f.id === selectedId.value) ?? null);

async function enter(f: Friend): Promise<void> {
  try {
    await joinFriend(f.id, f.name);
  } catch (e) {
    emit('error', isRpcError(e) ? t('errors.cannotJoin', { name: f.name, why: e.message }) : t('connect.genericError'));
  }
}
</script>

<template>
  <section class="friends">
    <header class="head">
      <h1 class="title">{{ $t('friends.title') }}</h1>
      <div class="tools">
        <UiButton variant="ghost" @click="emit('accept')">
          <Ticket :size="18" :stroke-width="1.75" aria-hidden="true" />
          {{ $t('friends.haveInvite') }}
        </UiButton>
        <UiButton variant="primary" @click="emit('invite')">
          <UserPlus :size="18" :stroke-width="1.75" aria-hidden="true" />
          {{ $t('friends.invite') }}
        </UiButton>
      </div>
    </header>

    <EmptyState v-if="social.friends.length === 0" :title="$t('friends.emptyTitle')" :body="$t('friends.emptyBody')">
      <UiButton variant="secondary" :size="48" @click="emit('invite')">{{ $t('friends.emptyAction') }}</UiButton>
    </EmptyState>

    <template v-else>
      <label class="search">
        <Search :size="16" :stroke-width="1.75" aria-hidden="true" />
        <span class="sr-only">{{ $t('friends.search') }}</span>
        <input v-model="query" type="search" :placeholder="$t('friends.search')" />
      </label>

      <section v-if="groups.playing.length" class="group">
        <h2 class="label">{{ $t('friends.groups.playing') }}</h2>
        <ul class="list">
          <FriendCard v-for="f in groups.playing" :key="f.id" :friend="f" :selected="f.id === selectedId" @open="selectedId = f.id" @join="enter(f)" />
        </ul>
      </section>
      <section v-if="groups.online.length" class="group">
        <h2 class="label">{{ $t('friends.groups.online') }}</h2>
        <ul class="list">
          <FriendCard v-for="f in groups.online" :key="f.id" :friend="f" :selected="f.id === selectedId" @open="selectedId = f.id" @join="enter(f)" />
        </ul>
      </section>
      <details v-if="groups.offline.length" class="group">
        <summary class="label">{{ $t('friends.groups.offline') }} · {{ groups.offline.length }}</summary>
        <ul class="list">
          <FriendCard v-for="f in groups.offline" :key="f.id" :friend="f" :selected="f.id === selectedId" @open="selectedId = f.id" @join="enter(f)" />
        </ul>
      </details>
    </template>

    <Teleport to="#detail-slot" defer>
      <FriendDetail v-if="selected" :friend="selected" @close="selectedId = null" @join="enter(selected)" />
    </Teleport>
  </section>
</template>

<style lang="scss" scoped>
.friends {
  display: grid;
  gap: $sp-5;
  max-width: 820px;

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

    .tools {
      display: flex;
      gap: $sp-2;
    }
  }

  .search {
    @include surface(2);
    display: flex;
    align-items: center;
    gap: $sp-2;
    height: $control-h;
    padding: 0 $sp-3;
    border-radius: $r-button;
    color: $text-3;

    &:focus-within {
      border-color: $accent-500;
    }

    input {
      flex: 1;
      border: 0;
      outline: none;
      background: none;
      color: $text-1;
      font: inherit;
    }
  }

  .group {
    display: grid;
    gap: $sp-2;

    .label {
      @include label;
    }

    summary {
      cursor: pointer;
      list-style: none;

      &:focus-visible {
        color: $accent-500;
      }
    }

    .list {
      display: grid;
      gap: $sp-2;
      margin: 0;
      padding: 0;
      list-style: none;
    }
  }
}
</style>
