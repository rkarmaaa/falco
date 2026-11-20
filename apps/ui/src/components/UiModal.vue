<script setup lang="ts">
// Modale sul <dialog> nativo: focus, Esc e sfondo inerte sono gestiti dal browser
import { onMounted, ref } from 'vue';
import { X } from 'lucide-vue-next';
import { useStageCover } from '@/stores/stage';

defineProps<{ title: string }>();
const emit = defineEmits<{ close: [] }>();
const dialog = ref<HTMLDialogElement | null>(null);

// Lo stream incorporato starebbe sopra la modale: si nasconde finché è aperta
useStageCover();
onMounted(() => dialog.value?.showModal?.());

// Clic sullo sfondo scuro: chiude. Conta solo se anche la pressione è partita fuori,
// così selezionare del testo e rilasciare oltre il bordo non chiude la modale.
let downOutside = false;
function outside(e: MouseEvent): boolean {
  const el = dialog.value;
  if (!el || e.target !== el) return false;
  const r = el.getBoundingClientRect();
  return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
}
function onPointerDown(e: PointerEvent): void {
  downOutside = outside(e);
}
function onClick(e: MouseEvent): void {
  if (downOutside && outside(e)) emit('close');
  downOutside = false;
}
</script>

<template>
  <dialog
    ref="dialog"
    class="modal"
    :aria-label="title"
    @close="emit('close')"
    @cancel.prevent="emit('close')"
    @pointerdown="onPointerDown"
    @click="onClick"
  >
    <header class="head">
      <h2 class="title">{{ title }}</h2>
      <button type="button" class="close" :aria-label="$t('app.close')" @click="emit('close')">
        <X :size="18" :stroke-width="1.75" />
      </button>
    </header>
    <div class="body">
      <slot />
    </div>
    <footer v-if="$slots.actions" class="actions">
      <slot name="actions" />
    </footer>
  </dialog>
</template>

<style lang="scss" scoped>
.modal {
  @include surface(1);
  width: min(520px, calc(100vw - #{$sp-8}));
  max-height: calc(100vh - #{$sp-9});
  padding: 0;
  border-radius: $r-modal;
  color: $text-1;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.35);

  &::backdrop {
    background: $scrim;
  }

  &[open] {
    animation: enter $t-modal $ease;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: $sp-5 $sp-6 0;

    .title {
      font-size: $fs-h2;
      line-height: 24px;
      font-weight: 600;
    }

    .close {
      @include reset-button;
      @include focus-ring;
      display: grid;
      place-items: center;
      width: 32px;
      height: 32px;
      border-radius: $r-button;
      color: $text-2;

      &:hover {
        background: $bg-3;
      }
    }
  }

  .body {
    display: grid;
    gap: $sp-4;
    padding: $sp-5 $sp-6;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: $sp-2;
    padding: 0 $sp-6 $sp-5;
  }
}

@keyframes enter {
  from {
    opacity: 0;
    transform: translateY(8px) scale(0.98);
  }
}
</style>
