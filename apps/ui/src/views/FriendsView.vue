<script setup lang="ts">
// Schermata principale (§13): richieste d'amicizia, ricerca, «In gioco ora», «Online», «Offline» chiuso
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Ticket, UserPlus } from 'lucide-vue-next';
import UiPage from '@/components/UiPage.vue';
import FriendRequests from '@/components/FriendRequests.vue';
import EmptyState from '@/components/EmptyState.vue';
import UiButton from '@/components/UiButton.vue';
import FriendCard from '@/components/FriendCard.vue';
import FriendDetail from '@/components/FriendDetail.vue';
import { allFriends, inviteFriend, social } from '@/stores/social';
import { core, joinFriend, setHostAvailable } from '@/stores/core';
import { attempt } from '@/stores/errors';
import { openAccept, openModal } from '@/stores/ui';
import type { Friend } from '@/ipc';

const { t } = useI18n();
const query = ref('');
const selectedId = ref<string | null>(null);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return allFriends().filter((f) => !q || f.name.toLowerCase().includes(q) || f.tag.toLowerCase().includes(q));
});
const groups = computed(() => ({
  playing: filtered.value.filter((f) => f.state === 'playing'),
  online: filtered.value.filter((f) => ['online', 'busy', 'pending'].includes(f.state)),
  offline: filtered.value.filter((f) => f.state === 'offline'),
}));
const selected = computed(() => social.friends.find((f) => f.id === selectedId.value) ?? null);

// «Invita» con un clic: avvia la condivisione e recapita l'invito tramite il Falco Hub
const invited = ref(new Set<string>());
async function invite(f: Friend): Promise<void> {
  // Prima la condivisione, poi l'invito: chi accetta subito trova l'host già in avvio
  if (core.host.state === 'stopped' || core.host.state === 'error') void attempt(t('errors.context.share'), () => setHostAvailable(true));
  const ok = await attempt(t('errors.context.invite', { name: f.name }), () => inviteFriend(f.id).then(() => true), { name: f.name });
  if (!ok) return;
  invited.value = new Set(invited.value).add(f.id);
  setTimeout(() => {
    const next = new Set(invited.value);
    next.delete(f.id);
    invited.value = next;
  }, 4000);
}

function open(f: Friend): void {
  if (!f.remote) selectedId.value = f.id;
}

function enter(f: Friend): void {
  void attempt(t('errors.context.join', { name: f.name }), () => joinFriend(f.id, f.name), { name: f.name });
}
</script>

<template>
  <UiPage class="friends" :title="$t('friends.title')">
    <template #actions>
      <UiButton variant="ghost" @click="openAccept()">
        <Ticket :size="18" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('friends.haveInvite') }}
      </UiButton>
      <UiButton variant="primary" @click="openModal('addFriend')">
        <UserPlus :size="18" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('addFriend.title') }}
      </UiButton>
    </template>

    <section v-if="social.friendRequests.incoming.length" class="group">
      <h2 class="label">{{ $t('addFriend.incoming') }} · {{ social.friendRequests.incoming.length }}</h2>
      <FriendRequests :requests="social.friendRequests.incoming" />
    </section>

    <EmptyState v-if="allFriends().length === 0" :title="$t('friends.emptyTitle')" :body="$t('friends.emptyBody')">
      <UiButton variant="secondary" :size="48" @click="openModal('addFriend')">{{ $t('friends.emptyAction') }}</UiButton>
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
          <FriendCard v-for="f in groups.playing" :key="f.id" :friend="f" :selected="f.id === selectedId" :invited="invited.has(f.id)" @open="open(f)" @join="enter(f)" @invite="invite(f)" />
        </ul>
      </section>
      <section v-if="groups.online.length" class="group">
        <h2 class="label">{{ $t('friends.groups.online') }}</h2>
        <ul class="list">
          <FriendCard v-for="f in groups.online" :key="f.id" :friend="f" :selected="f.id === selectedId" :invited="invited.has(f.id)" @open="open(f)" @join="enter(f)" @invite="invite(f)" />
        </ul>
      </section>
      <details v-if="groups.offline.length" class="group" open>
        <summary class="label">{{ $t('friends.groups.offline') }} · {{ groups.offline.length }}</summary>
        <ul class="list">
          <FriendCard v-for="f in groups.offline" :key="f.id" :friend="f" :selected="f.id === selectedId" :invited="invited.has(f.id)" @open="open(f)" @join="enter(f)" @invite="invite(f)" />
        </ul>
      </details>
    </template>

    <FriendDetail v-if="selected" :friend="selected" @close="selectedId = null" @join="(enter(selected), (selectedId = null))" />
  </UiPage>
</template>

<style lang="scss" scoped>
.friends {
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
      border-color: $accent-fg;
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
    gap: $section-gap;

    .label {
      @include label;
    }

    summary {
      cursor: pointer;
      list-style: none;

      &:focus-visible {
        color: $accent-fg;
      }
    }

    .list {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
      gap: $section-gap;
      margin: 0;
      padding: 0;
      list-style: none;
    }
  }
}
</style>
