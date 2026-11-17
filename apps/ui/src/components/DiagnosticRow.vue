<script setup lang="ts">
// Riga diagnostica: semaforo, titolo, valore misurato, azione. Mai solo colore.
import { computed } from 'vue';
import { CircleCheck, CircleDashed, CircleX, TriangleAlert } from 'lucide-vue-next';
import type { DoctorCheck } from '@/ipc';

const props = defineProps<{ check: DoctorCheck }>();
const icon = computed(
  () => ({ ok: CircleCheck, warn: TriangleAlert, fail: CircleX, pending: CircleDashed })[props.check.level],
);
</script>

<template>
  <li class="row" :class="check.level">
    <component :is="icon" class="light" :size="20" :stroke-width="1.75" aria-hidden="true" />
    <div class="main">
      <span class="title">{{ $t(`doctor.check.${check.id}`) }}</span>
      <span v-if="check.action" class="action">{{ $t(`doctor.action.${check.action}`) }}</span>
    </div>
    <span class="value">{{ check.value ?? $t(`doctor.level.${check.level}`) }}</span>
  </li>
</template>

<style lang="scss" scoped>
.row {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: $sp-3;
  padding: $sp-3 $sp-4;
  border-bottom: 1px solid $line;

  &:last-child {
    border-bottom: 0;
  }

  .main {
    display: grid;
    gap: 2px;
    min-width: 0;

    .title {
      font-weight: 600;
    }

    .action {
      font-size: $fs-small;
      line-height: 18px;
      color: $text-2;
    }
  }

  .value {
    font-size: $fs-small;
    color: $text-2;
    font-variant-numeric: tabular-nums;
  }

  &.ok .light {
    color: $ok;
  }

  &.warn .light {
    color: $busy;
  }

  &.fail .light {
    color: $danger;
  }

  &.pending {
    .light,
    .title,
    .value {
      color: $text-3;
    }
  }
}
</style>
