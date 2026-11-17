<script setup lang="ts" generic="T extends string">
// Scelta tra poche opzioni (tema, lingua, stato): radiogroup con frecce
const model = defineModel<T>({ required: true });
const props = defineProps<{ label: string; options: { value: T; label: string }[] }>();

function move(step: number): void {
  const i = props.options.findIndex((o) => o.value === model.value);
  const next = props.options[(i + step + props.options.length) % props.options.length];
  if (next) model.value = next.value;
}
</script>

<template>
  <div class="segmented" role="radiogroup" :aria-label="label" @keydown.left.prevent="move(-1)" @keydown.right.prevent="move(1)">
    <button
      v-for="o in options"
      :key="o.value"
      type="button"
      role="radio"
      class="option"
      :class="{ active: o.value === model }"
      :aria-checked="o.value === model"
      :tabindex="o.value === model ? 0 : -1"
      @click="model = o.value"
    >
      {{ o.label }}
    </button>
  </div>
</template>

<style lang="scss" scoped>
.segmented {
  @include surface(2);
  display: inline-flex;
  gap: 2px;
  padding: 3px;
  height: $control-h;
  border-radius: $r-button;

  .option {
    @include reset-button;
    @include focus-ring;
    padding: 0 $sp-3;
    border-radius: 7px;
    color: $text-2;
    font-size: $fs-small;
    font-weight: 600;
    transition:
      background $t-fast $ease,
      color $t-fast $ease;

    &:hover {
      color: $text-1;
    }

    &.active {
      background: $bg-3;
      color: $text-1;
    }
  }
}
</style>
