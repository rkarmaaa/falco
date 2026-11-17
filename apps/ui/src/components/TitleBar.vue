<script setup lang="ts">
// Barra del titolo trascinabile. Su macOS restano i semafori di sistema.
import { Minus, Square, X } from 'lucide-vue-next';
import { isTauri } from '@/ipc';

const isMac = navigator.userAgent.includes('Mac');
const showControls = isTauri() && !isMac;

async function win() {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  return getCurrentWindow();
}
</script>

<template>
  <header class="titlebar" :class="{ mac: isMac }" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>{{ $t('app.name') }}</span>
    <div v-if="showControls" class="controls">
      <button type="button" class="control" :aria-label="$t('app.minimize')" @click="win().then((w) => w.minimize())">
        <Minus :size="14" :stroke-width="1.75" />
      </button>
      <button type="button" class="control" :aria-label="$t('app.maximize')" @click="win().then((w) => w.toggleMaximize())">
        <Square :size="12" :stroke-width="1.75" />
      </button>
      <button type="button" class="control close" :aria-label="$t('app.close')" @click="win().then((w) => w.close())">
        <X :size="15" :stroke-width="1.75" />
      </button>
    </div>
  </header>
</template>

<style lang="scss" scoped>
.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: $titlebar-h;
  padding-left: $sp-4;
  background: $bg-1;
  border-bottom: 1px solid $line;

  .title {
    @include label;
  }

  .controls {
    display: flex;
    height: 100%;

    .control {
      @include reset-button;
      @include focus-ring;
      display: grid;
      place-items: center;
      width: 46px;
      height: 100%;
      color: $text-2;
      transition: background $t-fast $ease;

      &:hover {
        background: $bg-3;
        color: $text-1;
      }

      &.close:hover {
        background: $danger;
        color: $on-ember;
      }
    }
  }

  // Spazio per i semafori di macOS
  &.mac {
    padding-left: 80px;
  }
}
</style>
