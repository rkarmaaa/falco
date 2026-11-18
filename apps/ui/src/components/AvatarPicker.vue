<script setup lang="ts">
// Immagine del profilo: clic sull'avatar o su «Cambia», anteprima con fotocamera al passaggio.
import { ref } from 'vue';
import { Camera, Trash2 } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import UiButton from './UiButton.vue';

withDefaults(defineProps<{ name: string; src?: string | null; accent?: string; stacked?: boolean }>(), {
  src: null,
  accent: undefined,
  stacked: false,
});
const emit = defineEmits<{ pick: [file: File]; clear: [] }>();
const input = ref<HTMLInputElement | null>(null);

function changed(e: Event): void {
  const el = e.target as HTMLInputElement;
  const f = el.files?.[0];
  if (f) emit('pick', f);
  el.value = '';
}
</script>

<template>
  <div class="picker" :class="{ stacked }">
    <button type="button" class="face" :aria-label="$t('avatar.change')" @click="input?.click()">
      <UiAvatar :name="name || '?'" :src="src" :accent="accent" :size="96" />
      <span class="overlay" aria-hidden="true"><Camera :size="22" :stroke-width="1.75" /></span>
    </button>
    <div class="side">
      <div class="buttons">
        <UiButton variant="secondary" :size="32" @click="input?.click()">
          <Camera :size="16" :stroke-width="1.75" aria-hidden="true" />
          {{ src ? $t('avatar.change') : $t('avatar.add') }}
        </UiButton>
        <UiButton v-if="src" variant="ghost" :size="32" @click="emit('clear')">
          <Trash2 :size="16" :stroke-width="1.75" aria-hidden="true" />
          {{ $t('avatar.remove') }}
        </UiButton>
      </div>
      <p class="hint">{{ $t('avatar.hint') }}</p>
    </div>
    <input ref="input" class="sr-only" type="file" accept="image/*" tabindex="-1" @change="changed" />
  </div>
</template>

<style lang="scss" scoped>
.picker {
  display: flex;
  align-items: center;
  gap: $sp-5;

  &.stacked {
    flex-direction: column;
    gap: $sp-3;
    text-align: center;

    .side {
      justify-items: center;
    }
  }

  .face {
    @include reset-button;
    @include focus-ring;
    position: relative;
    border-radius: $r-pill;

    .overlay {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
      border-radius: $r-pill;
      background: $scrim;
      color: $text-1;
      opacity: 0;
      transition: opacity $t-fast $ease;
    }

    &:hover .overlay,
    &:focus-visible .overlay {
      opacity: 1;
    }
  }

  .side {
    display: grid;
    gap: $sp-2;

    .buttons {
      display: flex;
      gap: $sp-2;
    }

    .hint {
      @include message($text-3);
    }
  }
}
</style>
