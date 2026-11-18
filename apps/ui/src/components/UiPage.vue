<script setup lang="ts">
// Pagina di una vista: titolo, azioni a destra e contenuto con le stesse spaziature ovunque.
withDefaults(defineProps<{ title: string; narrow?: boolean }>(), { narrow: false });
</script>

<template>
  <section class="page" :class="{ narrow }">
    <header class="head">
      <h1 class="title">{{ title }}</h1>
      <slot name="meta" />
      <div v-if="$slots.actions" class="actions"><slot name="actions" /></div>
    </header>
    <slot />
  </section>
</template>

<style lang="scss" scoped>
.page {
  display: grid;
  align-content: start;
  gap: $page-gap;

  &.narrow {
    max-width: $page-narrow-w;
  }

  .head {
    display: flex;
    align-items: center;
    gap: $sp-4;
    // Stessa altezza con o senza pulsanti: il titolo non si sposta tra le viste
    min-height: $control-h;

    .title {
      @include title-h1;
    }

    .actions {
      display: flex;
      gap: $sp-2;
      margin-left: auto;
    }
  }
}
</style>
