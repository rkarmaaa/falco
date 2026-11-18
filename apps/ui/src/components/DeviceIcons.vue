<script setup lang="ts">
// Tastiera, mouse e controller: spenti se non concessi, accesi se concessi,
// luminosi mentre arriva input. Lo stato è scritto anche nel testo per i lettori di schermo.
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Gamepad2, Keyboard, Mouse } from 'lucide-vue-next';
import type { Device } from '@/stores/social';

const props = withDefaults(
  defineProps<{
    allowed: Record<Device, boolean>;
    active?: Partial<Record<Device, boolean>>;
    size?: number;
  }>(),
  { active: () => ({}), size: 15 },
);
const { t } = useI18n();

const items = computed(() =>
  (
    [
      ['keyboard', Keyboard],
      ['mouse', Mouse],
      ['controller', Gamepad2],
    ] as const
  ).map(([id, icon]) => ({
    id,
    icon,
    on: props.allowed[id],
    live: props.allowed[id] && !!props.active[id],
  })),
);
</script>

<template>
  <span class="devices">
    <span
      v-for="d in items"
      :key="d.id"
      class="device"
      :class="{ on: d.on, live: d.live }"
      :title="`${t(`devices.${d.id}`)} · ${d.on ? t('devices.allowed') : t('devices.denied')}`"
    >
      <component :is="d.icon" :size="size" :stroke-width="1.9" aria-hidden="true" />
      <span class="sr-only">{{ t(`devices.${d.id}`) }}: {{ d.on ? t('devices.allowed') : t('devices.denied') }}</span>
    </span>
  </span>
</template>

<style lang="scss" scoped>
.devices {
  display: inline-flex;
  gap: 2px;

  .device {
    display: grid;
    place-items: center;
    width: 24px;
    height: 22px;
    border-radius: 6px;
    color: $text-3;
    opacity: 0.38;
    transition:
      color $t-fast $ease,
      background $t-fast $ease,
      opacity $t-fast $ease,
      box-shadow $t-fast $ease;

    &.on {
      opacity: 1;
      color: $text-2;
    }

    // Input in arrivo: si illumina
    &.live {
      color: $live;
      background: $live-soft;
      box-shadow: 0 0 12px $live-glow;
    }
  }
}
</style>
