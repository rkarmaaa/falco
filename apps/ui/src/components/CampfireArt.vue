<script setup lang="ts">
// Illustrazione originale: falò stilizzato a tratto, per gli stati vuoti
defineProps<{ size?: number }>();
</script>

<template>
  <svg
    class="campfire"
    :width="size ?? 160"
    :height="size ?? 160"
    viewBox="0 0 160 160"
    fill="none"
    aria-hidden="true"
  >
    <circle class="halo" cx="80" cy="104" r="56" />
    <path
      class="flame"
      d="M80 30c6 18 26 30 26 56 0 18-12 30-26 30S54 104 54 86c0-14 8-22 13-30 2 9 7 13 11 15-2-16 0-30 2-41z"
    />
    <path class="core" d="M80 74c3 9 12 14 12 26 0 8-5 13-12 13s-12-5-12-13c0-6 3-10 6-13 1 4 3 6 5 7-1-7 0-14 1-20z" />
    <path class="log" d="M44 128l72-20M116 128l-72-20" />
    <path class="spark" d="M112 48v6M109 51h6M50 58v4M48 60h4M120 76v4M118 78h4" />
  </svg>
</template>

<style lang="scss" scoped>
.campfire {
  .halo {
    fill: $ember-glow;
    opacity: 0.35;
  }

  .flame {
    stroke: $ember-500;
    stroke-width: 3;
    stroke-linejoin: round;
  }

  .core {
    stroke: $ember-400;
    stroke-width: 2.5;
    stroke-linejoin: round;
  }

  .log {
    stroke: $text-2;
    stroke-width: 7;
    stroke-linecap: round;
  }

  .spark {
    stroke: $ember-400;
    stroke-width: 2;
    stroke-linecap: round;
  }
}
</style>
