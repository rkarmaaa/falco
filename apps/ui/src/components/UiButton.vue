<script setup lang="ts">
// Pulsante: primario accento, secondario superficie, fantasma, pericolo
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 32 | 40 | 48;
    icon?: boolean;
    disabled?: boolean;
    type?: 'button' | 'submit';
  }>(),
  { variant: 'secondary', size: 40, icon: false, disabled: false, type: 'button' },
);
</script>

<template>
  <button
    class="button"
    :class="[variant, `h${size}`, { icon }]"
    :type="type"
    :disabled="disabled"
  >
    <slot />
  </button>
</template>

<style lang="scss" scoped>
.button {
  @include reset-button;
  @include focus-ring;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: $sp-2;
  padding: 0 $sp-4;
  border-radius: $r-button;
  font-weight: 600;
  font-size: $fs-body;
  white-space: nowrap;
  transition:
    background $t-fast $ease,
    opacity $t-fast $ease,
    transform $t-fast $ease;

  &.h32 {
    height: 32px;
    padding: 0 $sp-3;
    font-size: $fs-small;
  }

  &.h40 {
    height: 40px;
  }

  &.h48 {
    height: 48px;
    padding: 0 $sp-6;
  }

  &.icon {
    padding: 0;
    aspect-ratio: 1;
  }

  &.primary {
    background: $accent-500;
    color: $on-accent;

    &:hover {
      background: $accent-400;
    }

    &:active {
      background: $accent-600;
    }
  }

  &.secondary {
    @include surface(2);
    color: $text-1;

    &:hover {
      background: $bg-3;
    }
  }

  &.ghost {
    color: $text-2;

    &:hover {
      background: $bg-3;
      color: $text-1;
    }
  }

  &.danger {
    background: transparent;
    color: $danger;
    // Bordo tenue a riposo: il rosso pieno resta per il passaggio del mouse
    border: 1px solid $danger-line;

    &:hover {
      background: $danger;
      color: $on-accent;
    }
  }

  &:active {
    transform: scale(0.98);
  }

  &:disabled {
    opacity: 0.45;
    pointer-events: none;
  }
}
</style>
