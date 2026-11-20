<script setup lang="ts">
// Finestra del pulsantino (index.html#pill): legge la sessione dal core e passa
// le azioni alla finestra principale.
import { onMounted, ref } from 'vue';
import StreamPill from '@/components/StreamPill.vue';
import { getTransport, rpc, type SessionStats, type StateSnapshot } from '@/ipc';

const name = ref('');
const stats = ref<SessionStats | null>(null);

async function action(a: 'exit' | 'leave'): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('pill_action', { action: a });
}

// Nascosta: resta una striscia sul bordo alto; il cursore lassù la fa scendere
const hidden = ref(false);
const peek = ref(false);
let leaveTimer: ReturnType<typeof setTimeout> | undefined;

async function size(collapsed: boolean): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('pill_collapse', { collapsed }).catch(() => {});
}

async function hide(): Promise<void> {
  hidden.value = true;
  peek.value = false;
  await size(true);
}

async function enter(): Promise<void> {
  clearTimeout(leaveTimer);
  if (!hidden.value || peek.value) return;
  await size(false);
  peek.value = true;
}

function leaveWindow(): void {
  if (!hidden.value) return;
  clearTimeout(leaveTimer);
  leaveTimer = setTimeout(async () => {
    peek.value = false;
    // Si ritira prima l'animazione, poi la finestra
    setTimeout(() => void size(true), 220);
  }, 700);
}

function pin(): void {
  hidden.value = false;
  peek.value = false;
}

onMounted(async () => {
  document.body.classList.add('pill');
  // La finestra viene nascosta e riusata: ricompare sempre aperta, come la sua dimensione
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') pin();
  });
  const snap = await rpc<StateSnapshot>('state.snapshot').catch(() => null);
  name.value = snap?.session?.name ?? '';
  const t = await getTransport();
  t.onEvent((e) => {
    const p = e.params as Record<string, unknown>;
    if (e.method === 'session.stats' && p.phase === 'stats') stats.value = p as unknown as SessionStats;
    if (e.method === 'session.started') name.value = String(p.name ?? '');
  });
});
</script>

<template>
  <div class="wrap" :class="{ hidden, peek }" @mouseenter="enter" @mouseleave="leaveWindow">
    <StreamPill
      :name="name"
      :stats="stats"
      :hidden="hidden"
      @exit="action('exit')"
      @leave="action('leave')"
      @hide="hidden ? pin() : hide()"
    />
  </div>
</template>

<style lang="scss" scoped>
.wrap {
  display: grid;
  justify-items: center;
  align-content: start;
  height: 100%;
  padding-top: 12px;
  transition: transform $t-panel $ease;

  // Nascosta: scivola fuori dal bordo alto e rientra al passaggio del cursore
  &.hidden:not(.peek) {
    transform: translateY(-110%);
  }
}
</style>
