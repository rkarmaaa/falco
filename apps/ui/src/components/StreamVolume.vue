<script setup lang="ts">
// Volume dello stream per chi guarda: solo l'audio di Moonlight, non quello del sistema.
// Si ricorda tra le sessioni e si applica appena l'audio dello stream parte.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Volume1, Volume2, VolumeX } from 'lucide-vue-next';
import { isTauri } from '@/ipc';

const props = defineProps<{ pid: number }>();
const KEY = 'falco.streamVolume';

function load(): { volume: number; muted: boolean } {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '');
    if (typeof v.volume === 'number') return { volume: v.volume, muted: !!v.muted };
  } catch {
    // Niente di salvato
  }
  return { volume: 100, muted: false };
}

const saved = load();
const volume = ref(saved.volume);
const muted = ref(saved.muted);
const icon = computed(() => (muted.value || volume.value === 0 ? VolumeX : volume.value < 50 ? Volume1 : Volume2));

async function apply(): Promise<boolean> {
  if (!isTauri()) return true;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('stream_volume', { pid: props.pid, volume: volume.value / 100, muted: muted.value }).catch(() => false);
}

// La sessione audio di Moonlight compare qualche secondo dopo l'avvio: si riprova per 30 s
let retry: ReturnType<typeof setInterval> | undefined;
function applyUntilReady(): void {
  clearInterval(retry);
  let tries = 0;
  retry = setInterval(async () => {
    if ((await apply()) || ++tries >= 15) clearInterval(retry);
  }, 2000);
  void apply();
}

watch([volume, muted], () => {
  try {
    localStorage.setItem(KEY, JSON.stringify({ volume: volume.value, muted: muted.value }));
  } catch {
    // Archivio non disponibile: vale solo per questa sessione
  }
  void apply();
});
watch(() => props.pid, applyUntilReady);
onMounted(applyUntilReady);
onBeforeUnmount(() => clearInterval(retry));
</script>

<template>
  <div class="volume" :class="{ muted }">
    <button
      type="button"
      class="mute"
      :aria-pressed="muted"
      :aria-label="muted ? $t('session.unmute') : $t('session.mute')"
      :title="muted ? $t('session.unmute') : $t('session.mute')"
      @click="muted = !muted"
    >
      <component :is="icon" :size="18" :stroke-width="1.9" aria-hidden="true" />
    </button>
    <input
      v-model.number="volume"
      class="slider"
      type="range"
      min="0"
      max="100"
      step="1"
      :aria-label="$t('session.volume')"
      :style="{ '--fill': `${muted ? 0 : volume}%` }"
      @input="muted = false"
    />
  </div>
</template>

<style lang="scss" scoped>
.volume {
  display: flex;
  align-items: center;
  gap: $sp-1;
  height: $control-h;
  padding: 0 $sp-2 0 $sp-1;
  border-radius: $r-button;

  .mute {
    @include reset-button;
    @include focus-ring;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: $r-field;
    color: $text-2;

    &:hover {
      background: $bg-3;
      color: $text-1;
    }
  }

  &.muted .mute {
    color: $danger;
  }

  .slider {
    @include focus-ring;
    width: 96px;
    height: 4px;
    appearance: none;
    border-radius: $r-pill;
    background: linear-gradient(to right, $accent-fg var(--fill), $bg-4 var(--fill));
    cursor: pointer;

    &::-webkit-slider-thumb {
      appearance: none;
      width: 14px;
      height: 14px;
      border-radius: $r-pill;
      background: $text-1;
      box-shadow: $shadow-1;
    }
  }
}
</style>
