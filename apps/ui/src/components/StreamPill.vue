<script setup lang="ts">
// Pulsantino sopra lo stream a schermo intero: discreto (quasi trasparente finché non
// ci passi sopra), con le azioni essenziali. Vive in una finestra sua, sempre in primo piano.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { EyeOff, LogOut, Minimize2, Pin } from 'lucide-vue-next';
import type { SessionStats } from '@/ipc';

const props = defineProps<{ name: string; stats: SessionStats | null; hidden?: boolean }>();
const emit = defineEmits<{ exit: []; leave: []; hide: [] }>();
const { t } = useI18n();
const awake = ref(true);
const confirmLeave = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;

function wake(): void {
  awake.value = true;
  clearTimeout(timer);
  timer = setTimeout(() => {
    awake.value = false;
    confirmLeave.value = false;
  }, 2600);
}
onMounted(wake);
onBeforeUnmount(() => clearTimeout(timer));

const level = computed(() => {
  const ms = props.stats?.rttMs ?? 0;
  return ms < 30 ? 'good' : ms <= 60 ? 'fair' : 'bad';
});

function leave(): void {
  // Due clic: uscire per sbaglio dallo stream a schermo intero è fastidioso
  if (!confirmLeave.value) {
    confirmLeave.value = true;
    wake();
    return;
  }
  emit('leave');
}
</script>

<template>
  <div class="pill" :class="{ awake }" role="toolbar" :aria-label="t('pill.label')" @mouseenter="wake" @mousemove="wake" @focusin="wake">
    <span class="live" aria-hidden="true" />
    <span class="name">{{ name }}</span>
    <span v-if="stats" class="ping" :class="level">{{ Math.round(stats.rttMs) }} ms</span>
    <span class="sep" aria-hidden="true" />
    <button
      type="button"
      class="btn icon"
      :title="hidden ? t('pill.pin') : t('pill.hide')"
      :aria-label="hidden ? t('pill.pin') : t('pill.hide')"
      @click="emit('hide')"
    >
      <Pin v-if="hidden" :size="15" :stroke-width="2" aria-hidden="true" />
      <EyeOff v-else :size="15" :stroke-width="2" aria-hidden="true" />
    </button>
    <button type="button" class="btn" :title="t('pill.exit')" @click="emit('exit')">
      <Minimize2 :size="15" :stroke-width="2" aria-hidden="true" />
      <span>{{ t('pill.exit') }}</span>
    </button>
    <button type="button" class="btn danger" :class="{ confirm: confirmLeave }" :title="t('session.leave')" @click="leave">
      <LogOut :size="15" :stroke-width="2" aria-hidden="true" />
      <span>{{ confirmLeave ? t('pill.confirmLeave') : t('session.leave') }}</span>
    </button>
  </div>
</template>

<style lang="scss" scoped>
.pill {
  display: inline-flex;
  align-items: center;
  gap: $sp-2;
  height: 40px;
  padding: 0 4px 0 $sp-3;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: $r-pill;
  background: rgba(10, 12, 22, 0.9);
  backdrop-filter: blur(12px);
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.5);
  color: $text-1;
  font-size: $fs-small;
  // Più leggibile anche a riposo
  opacity: 0.72;
  transform: translateY(-2px) scale(0.97);
  transition:
    opacity $t-panel $ease,
    transform $t-panel $ease;

  &.awake,
  &:hover,
  &:focus-within {
    opacity: 1;
    transform: none;
  }

  .live {
    width: 7px;
    height: 7px;
    border-radius: $r-pill;
    background: $live;
    box-shadow: 0 0 8px $live-glow;
  }

  .name {
    @include truncate;
    max-width: 110px;
    font-weight: 650;
  }

  .ping {
    font-weight: 650;
    font-variant-numeric: tabular-nums;

    &.good {
      color: $ok;
    }

    &.fair {
      color: $busy;
    }

    &.bad {
      color: $danger;
    }
  }

  .sep {
    width: 1px;
    height: 18px;
    background: rgba(255, 255, 255, 0.12);
  }

  .btn {
    @include reset-button;
    @include focus-ring;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 $sp-3;
    border-radius: $r-pill;
    color: $text-2;
    font-weight: 550;
    white-space: nowrap;

    &:hover {
      background: rgba(255, 255, 255, 0.08);
      color: $text-1;
    }

    &.danger:hover,
    &.danger.confirm {
      background: $danger;
      color: $on-accent;
    }
  }
}
</style>
