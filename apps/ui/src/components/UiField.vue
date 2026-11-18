<script setup lang="ts">
// Campo di testo: altezza 40, focus con anello d'accento
const model = defineModel<string>({ required: true });
defineProps<{ label: string; placeholder?: string; inputmode?: 'text' | 'numeric'; maxlength?: number }>();
</script>

<template>
  <label class="field">
    <span class="label">{{ label }}</span>
    <input
      v-model="model"
      class="input"
      :placeholder="placeholder"
      :inputmode="inputmode ?? 'text'"
      :maxlength="maxlength"
      spellcheck="false"
      autocomplete="off"
    />
  </label>
</template>

<style lang="scss" scoped>
.field {
  display: grid;
  gap: $sp-1;
  min-width: 0;

  .label {
    @include label;
  }

  .input {
    @include surface(2);
    height: $control-h;
    padding: 0 $sp-3;
    border-radius: $r-field;
    color: $text-1;
    font: inherit;
    outline: none;
    transition: border-color $t-fast $ease;

    &::placeholder {
      color: $text-3;
    }

    &:focus-visible {
      border-color: $accent-fg;
      box-shadow: 0 0 0 2px $accent-glow;
    }
  }
}
</style>
