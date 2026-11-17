<script setup lang="ts">
// Pillola ping: verde < 30 ms, ambra 30–60, rosso > 60; etichetta diretto/relay
import { computed } from 'vue';

const props = defineProps<{ ms: number; path: 'direct' | 'relay' }>();
const level = computed(() => (props.ms < 30 ? 'good' : props.ms <= 60 ? 'fair' : 'bad'));
</script>

<template>
  <span class="ping" :class="level">
    <span class="dot" aria-hidden="true" />
    <span class="ms">{{ Math.round(ms) }} ms</span>
    <span class="path">{{ $t(`ping.${path}`) }}</span>
  </span>
</template>

<style lang="scss" scoped>
.ping {
  display: inline-flex;
  align-items: center;
  gap: $sp-2;
  height: 24px;
  padding: 0 $sp-3;
  border-radius: $r-pill;
  background: $bg-3;
  font-size: $fs-small;
  font-variant-numeric: tabular-nums;

  .dot {
    width: 8px;
    height: 8px;
    border-radius: $r-pill;
    background: var(--c);
  }

  .path {
    color: $text-2;
  }

  &.good {
    --c: #{$ok};
  }

  &.fair {
    --c: #{$busy};
  }

  &.bad {
    --c: #{$danger};
  }
}
</style>
