<script setup lang="ts">
// Avatar con anello di stato; monogramma sul colore accent se manca l'immagine
import { computed } from 'vue';

const props = withDefaults(
  defineProps<{
    name: string;
    src?: string | null;
    accent?: string;
    size?: 24 | 32 | 40 | 56 | 96;
    status?: 'online' | 'busy' | 'offline' | 'playing' | null;
  }>(),
  { src: null, accent: '#FF6A2B', size: 40, status: null },
);

const initials = computed(() =>
  props.name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => [...w][0]?.toUpperCase() ?? '')
    .join(''),
);
</script>

<template>
  <span
    class="avatar"
    :class="[`s${size}`, status]"
    :style="{ '--accent': accent }"
    role="img"
    :aria-label="name"
  >
    <img v-if="src" :src="src" alt="" loading="lazy" :width="size" :height="size" />
    <span v-else class="monogram" aria-hidden="true">{{ initials }}</span>
  </span>
</template>

<style lang="scss" scoped>
.avatar {
  @include avatar(40px);
  position: relative;
  display: inline-grid;
  place-items: center;
  background: var(--accent);
  color: $on-accent;
  font-weight: 650;

  img {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    object-fit: cover;
  }

  .monogram {
    line-height: 1;
  }

  // Anello di stato da 2 px con 2 px di stacco
  &.online,
  &.busy,
  &.playing {
    box-shadow: 0 0 0 2px $bg-1, 0 0 0 4px var(--ring);
  }

  &.online {
    --ring: #{$ok};
  }

  &.busy {
    --ring: #{$busy};
  }

  &.playing {
    --ring: #{$accent-fg};
    @include glow;
  }

  &.offline {
    filter: grayscale(1);
    opacity: 0.6;
  }

  &.s24 {
    @include avatar(24px);
    font-size: 10px;
  }

  &.s32 {
    @include avatar(32px);
    font-size: 12px;
  }

  &.s40 {
    font-size: 14px;
  }

  &.s56 {
    @include avatar(56px);
    font-size: 20px;
  }

  &.s96 {
    @include avatar(96px);
    font-size: 34px;
  }
}
</style>
