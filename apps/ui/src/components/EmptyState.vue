<script setup lang="ts">
import CampfireArt from './CampfireArt.vue';

defineProps<{ title: string; body?: string }>();
</script>

<template>
  <section class="empty">
    <CampfireArt :size="144" />
    <h2 class="title">{{ title }}</h2>
    <p v-if="body" class="body">{{ body }}</p>
    <div v-if="$slots.default" class="actions">
      <slot />
    </div>
  </section>
</template>

<style lang="scss" scoped>
.empty {
  display: grid;
  justify-items: center;
  gap: $sp-3;
  max-width: 420px;
  margin: $sp-9 auto;
  text-align: center;

  .title {
    font-size: $fs-display;
    line-height: 38px;
    font-weight: 700;
    letter-spacing: -0.02em;

    @include mq(sm) {
      font-size: $fs-h1;
      line-height: 30px;
    }
  }

  .body {
    color: $text-2;
  }

  .actions {
    margin-top: $sp-3;
  }
}
</style>
