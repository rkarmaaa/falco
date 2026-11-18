<script setup lang="ts">
// Menu contestuale (tasto destro o clic): si apre nel punto indicato, resta nella finestra,
// si chiude con Esc o con un clic fuori. Copre il palco dello stream mentre è aperto.
import { nextTick, onMounted, ref } from 'vue';
import { onClickOutside, onKeyStroke } from '@vueuse/core';
import { useStageCover } from '@/stores/stage';

const props = defineProps<{ x: number; y: number; label: string }>();
const emit = defineEmits<{ close: [] }>();
const menu = ref<HTMLElement | null>(null);
const pos = ref({ left: props.x, top: props.y });

useStageCover();
onClickOutside(menu, () => emit('close'));
onKeyStroke('Escape', () => emit('close'));

onMounted(async () => {
  await nextTick();
  const el = menu.value;
  if (!el) return;
  const r = el.getBoundingClientRect();
  // Si apre verso l'alto se non c'è spazio sotto (la barra azioni è in fondo)
  pos.value = {
    left: Math.max(8, Math.min(props.x, window.innerWidth - r.width - 8)),
    top: props.y + r.height > window.innerHeight - 8 ? Math.max(8, props.y - r.height) : props.y,
  };
  el.querySelector<HTMLElement>('[role^="menuitem"]')?.focus();
});

function onKey(e: KeyboardEvent): void {
  const items = [...(menu.value?.querySelectorAll<HTMLElement>('[role^="menuitem"]') ?? [])];
  const i = items.indexOf(document.activeElement as HTMLElement);
  if (e.key === 'ArrowDown') items[(i + 1) % items.length]?.focus();
  else if (e.key === 'ArrowUp') items[(i - 1 + items.length) % items.length]?.focus();
  else return;
  e.preventDefault();
}
</script>

<template>
  <Teleport to="body">
    <div
      ref="menu"
      class="menu"
      role="menu"
      :aria-label="label"
      :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
      @keydown="onKey"
      @contextmenu.prevent
    >
      <slot />
    </div>
  </Teleport>
</template>

<style lang="scss" scoped>
.menu {
  position: fixed;
  z-index: 50;
  display: grid;
  min-width: 240px;
  padding: $sp-1;
  border: 1px solid $line-strong;
  border-radius: $r-card;
  background: rgba(25, 29, 49, 0.96);
  box-shadow: $shadow-3;
  backdrop-filter: blur(14px);
  animation: pop $t-panel $ease;

  :slotted(.head) {
    display: grid;
    gap: 2px;
    padding: $sp-2 $sp-3 $sp-2;

    strong {
      font-weight: 650;
    }

    span {
      font-size: $fs-small;
      color: $text-3;
    }
  }

  :slotted(hr) {
    width: 100%;
    height: 1px;
    margin: $sp-1 0;
    border: 0;
    background: $line;
  }

  :slotted(.item) {
    @include reset-button;
    display: flex;
    align-items: center;
    gap: $sp-3;
    height: 36px;
    padding: 0 $sp-3;
    border-radius: $r-button;
    color: $text-1;
    text-align: left;
    outline: none;

    svg {
      flex: 0 0 auto;
      color: $text-2;
    }

    .grow {
      flex: 1;
    }

    .check {
      color: $live;
    }

    .hint {
      font-size: $fs-small;
      color: $text-3;
    }

    &:hover,
    &:focus-visible {
      background: $bg-4;
    }

    &.danger {
      color: $danger;

      svg {
        color: $danger;
      }

      &:hover,
      &:focus-visible {
        background: $danger-soft;
      }
    }
  }
}

@keyframes pop {
  from {
    opacity: 0;
    transform: translateY(4px) scale(0.98);
  }
}
</style>
