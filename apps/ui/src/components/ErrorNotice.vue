<script setup lang="ts">
// Avviso di errore: messaggio in tre parti, «Copia» e «Debug». In linea o come toast.
import { ref } from 'vue';
import { Bug, Check, CircleAlert, Copy, X } from 'lucide-vue-next';
import { describe, openDebug, type AppError } from '@/stores/errors';
import { copyText } from '@/utils/clipboard';

const props = defineProps<{ error: AppError; dismissible?: boolean }>();
const emit = defineEmits<{ dismiss: [] }>();
const copied = ref(false);

async function copy(): Promise<void> {
  copied.value = await copyText(describe(props.error));
  setTimeout(() => (copied.value = false), 2000);
}
</script>

<template>
  <div class="notice" role="alert">
    <CircleAlert class="icon" :size="18" :stroke-width="1.75" aria-hidden="true" />
    <p class="text">{{ error.friendly }}</p>
    <div class="actions">
      <button type="button" class="act" :aria-label="$t('errors.copy')" :title="$t('errors.copy')" @click="copy">
        <Check v-if="copied" :size="16" :stroke-width="1.75" aria-hidden="true" />
        <Copy v-else :size="16" :stroke-width="1.75" aria-hidden="true" />
        <span>{{ copied ? $t('errors.copied') : $t('errors.copy') }}</span>
      </button>
      <button type="button" class="act" :title="$t('errors.debug')" @click="openDebug(error)">
        <Bug :size="16" :stroke-width="1.75" aria-hidden="true" />
        <span>{{ $t('errors.debug') }}</span>
      </button>
      <button v-if="dismissible" type="button" class="act close" :aria-label="$t('app.close')" @click="emit('dismiss')">
        <X :size="16" :stroke-width="1.75" aria-hidden="true" />
      </button>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.notice {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: $sp-2 $sp-3;
  padding: $sp-3 $sp-4;
  border: 1px solid $danger;
  border-radius: $r-card;
  background: $bg-2;
  max-width: 520px;

  .icon {
    color: $danger;
    margin-top: 1px;
  }

  .text {
    font-size: $fs-small;
    line-height: 18px;
    color: $text-1;
    user-select: text;
  }

  .actions {
    grid-column: 2;
    display: flex;
    gap: $sp-1;

    .act {
      @include reset-button;
      @include focus-ring;
      display: inline-flex;
      align-items: center;
      gap: $sp-1;
      height: 28px;
      padding: 0 $sp-2;
      border-radius: $r-field;
      font-size: $fs-small;
      font-weight: 600;
      color: $text-2;

      &:hover {
        background: $bg-3;
        color: $text-1;
      }

      &.close {
        margin-left: auto;
      }
    }
  }
}
</style>
