<script setup lang="ts">
// Interruttore accessibile: role=switch, uso da tastiera con Spazio/Invio
const model = defineModel<boolean>({ required: true });
defineProps<{ label: string; hint?: string; disabled?: boolean }>();
</script>

<template>
  <label class="switch" :class="{ disabled }">
    <span class="text">
      <span class="label">{{ label }}</span>
      <span v-if="hint" class="hint">{{ hint }}</span>
    </span>
    <button
      type="button"
      role="switch"
      class="track"
      :class="{ on: model }"
      :aria-checked="model"
      :disabled="disabled"
      @click="model = !model"
    >
      <span class="thumb" />
    </button>
  </label>
</template>

<style lang="scss" scoped>
.switch {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: $sp-4;
  min-height: $control-h;

  .text {
    display: grid;
    gap: 2px;

    .label {
      font-weight: 600;
    }

    .hint {
      font-size: $fs-small;
      line-height: 18px;
      color: $text-2;
    }
  }

  .track {
    @include reset-button;
    @include focus-ring;
    position: relative;
    flex: 0 0 44px;
    height: 24px;
    border-radius: $r-pill;
    background: $bg-3;
    border: 1px solid $line;
    transition: background $t-fast $ease;

    .thumb {
      position: absolute;
      top: 3px;
      left: 3px;
      width: 16px;
      height: 16px;
      border-radius: $r-pill;
      background: $text-2;
      transition:
        transform $t-fast $ease,
        background $t-fast $ease;
    }

    &.on {
      background: $accent-500;
      border-color: $accent-500;

      .thumb {
        transform: translateX(20px);
        background: $on-accent;
      }
    }
  }

  &.disabled {
    opacity: 0.5;
  }
}
</style>
