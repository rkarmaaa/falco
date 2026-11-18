<script setup lang="ts">
// Barra laterale da 72 px: quattro sezioni, in basso avatar e stato
import type { Component } from 'vue';
import { Gamepad2, Settings, Ticket, Users } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import { core } from '@/stores/core';
import { social } from '@/stores/social';

export type Section = 'friends' | 'host' | 'invites' | 'settings';

const model = defineModel<Section>({ required: true });

const items: { id: Section; icon: Component }[] = [
  { id: 'friends', icon: Users },
  { id: 'host', icon: Gamepad2 },
  { id: 'invites', icon: Ticket },
  { id: 'settings', icon: Settings },
];
</script>

<template>
  <nav class="sidebar" :aria-label="$t('app.name')">
    <ul class="items">
      <li v-for="item in items" :key="item.id">
        <button
          type="button"
          class="item"
          :class="{ active: model === item.id }"
          :aria-current="model === item.id ? 'page' : undefined"
          :title="$t(`nav.${item.id}`)"
          @click="model = item.id"
        >
          <component :is="item.icon" :size="22" :stroke-width="1.75" aria-hidden="true" />
          <span class="sr-only">{{ $t(`nav.${item.id}`) }}</span>
        </button>
      </li>
    </ul>
    <button
      type="button"
      class="me"
      :title="`${$t('nav.me')} · ${$t(`status.${core.snapshot?.status ?? 'online'}`)}`"
      @click="model = 'settings'"
    >
      <UiAvatar
        :name="social.profile.name ?? '?'"
        :src="social.profile.avatarUrl"
        :accent="social.profile.accent"
        :size="40"
        :status="core.snapshot?.status === 'busy' ? 'busy' : core.snapshot?.status === 'invisible' ? 'offline' : 'online'"
      />
    </button>
  </nav>
</template>

<style lang="scss" scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: space-between;
  width: $sidebar-w;
  padding: $sp-4 0;
  background: $bg-1;
  border-right: 1px solid $line;

  .items {
    display: grid;
    gap: $sp-2;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .item {
    @include reset-button;
    @include focus-ring;
    position: relative;
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: $r-card;
    color: $text-3;
    transition:
      background $t-fast $ease,
      color $t-fast $ease;

    &:hover {
      background: $bg-3;
      color: $text-1;
    }

    // Indicatore della sezione attiva: barra accento a sinistra
    &.active {
      background: $bg-3;
      color: $accent-fg;

      &::before {
        content: '';
        position: absolute;
        left: -12px;
        width: 3px;
        height: 20px;
        border-radius: $r-pill;
        background: $accent-fg;
      }
    }
  }

  .me {
    @include reset-button;
    @include focus-ring;
    border-radius: $r-pill;
  }
}
</style>
