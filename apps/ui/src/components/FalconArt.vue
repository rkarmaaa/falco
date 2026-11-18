<script setup lang="ts">
// Illustrazione originale: falco in planata a tratto, per gli stati vuoti
defineProps<{ size?: number }>();
</script>

<template>
  <svg class="falcon" :width="size ?? 160" :height="size ?? 160" viewBox="0 0 160 160" fill="none" aria-hidden="true">
    <circle class="halo" cx="80" cy="84" r="56" />
    <!-- Ali tese e corpo affusolato -->
    <path
      class="wing"
      d="M80 78 C 66 66 46 58 18 58 C 34 66 46 74 54 84 C 62 88 72 88 80 86 C 88 88 98 88 106 84 C 114 74 126 66 142 58 C 114 58 94 66 80 78 Z"
    />
    <path class="body" d="M80 70 C 74 74 72 82 74 92 C 76 102 78 110 80 118 C 82 110 84 102 86 92 C 88 82 86 74 80 70 Z" />
    <path class="tail" d="M74 112 L 80 128 L 86 112" />
    <circle class="eye" cx="80" cy="76" r="2.2" />
    <path class="trail" d="M30 112 H 58 M24 122 H 46 M104 112 H 132 M116 122 H 138" />
  </svg>
</template>

<style lang="scss" scoped>
.falcon {
  .halo {
    fill: $accent-glow;
    opacity: 0.3;
  }

  .wing,
  .body {
    stroke: $accent-fg;
    stroke-width: 2.5;
    stroke-linejoin: round;
  }

  .body {
    fill: $bg-2;
  }

  .tail {
    stroke: $accent-fg;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .eye {
    fill: $eye;
  }

  .trail {
    stroke: $text-3;
    stroke-width: 2;
    stroke-linecap: round;
    opacity: 0.6;
  }
}
</style>
