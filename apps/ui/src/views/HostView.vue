<script setup lang="ts">
import { computed } from 'vue';
import { Monitor } from 'lucide-vue-next';
import UiSwitch from '@/components/UiSwitch.vue';
import { core, setHostAvailable } from '@/stores/core';

const available = computed({
  get: () => core.snapshot?.hostAvailable ?? false,
  set: (v: boolean) => void setHostAvailable(v),
});
</script>

<template>
  <section class="host">
    <h1 class="title">{{ $t('host.title') }}</h1>

    <div class="panel" :class="{ live: available }">
      <UiSwitch v-model="available" :label="$t('host.available')" :hint="$t('host.availableHint')" />
    </div>

    <h2 class="group">{{ $t('host.apps') }}</h2>
    <ul class="apps">
      <li class="app">
        <span class="cover"><Monitor :size="28" :stroke-width="1.75" aria-hidden="true" /></span>
        <span class="name">{{ $t('host.desktop') }}</span>
      </li>
    </ul>

    <p class="session">{{ $t('host.noSession') }}</p>
  </section>
</template>

<style lang="scss" scoped>
.host {
  display: grid;
  gap: $sp-4;
  max-width: 720px;

  .title {
    font-size: $fs-h1;
    line-height: 30px;
    font-weight: 650;
  }

  .panel {
    @include surface(2);
    padding: $sp-4 $sp-5;
    border-radius: $r-card;
    transition: box-shadow $t-panel $ease;

    // Disponibile: stato vivo, bordo brace
    &.live {
      box-shadow: inset 0 0 0 1px $ember-500;
    }
  }

  .group {
    @include label;
    margin-top: $sp-4;
  }

  .apps {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: $sp-3;
    margin: 0;
    padding: 0;
    list-style: none;

    .app {
      @include surface(2);
      display: grid;
      gap: $sp-2;
      padding: $sp-2;
      border-radius: $r-card;

      .cover {
        display: grid;
        place-items: center;
        aspect-ratio: 3 / 4;
        border-radius: 10px;
        background: $bg-3;
        color: $text-2;
      }

      .name {
        @include truncate;
        padding: 0 $sp-1 $sp-1;
        font-weight: 600;
      }
    }
  }

  .session {
    color: $text-3;
    font-size: $fs-small;
  }
}
</style>
