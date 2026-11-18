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

onMounted(async () => {
  document.body.classList.add('pill');
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
  <div class="wrap" data-tauri-drag-region>
    <StreamPill :name="name" :stats="stats" @exit="action('exit')" @leave="action('leave')" />
  </div>
</template>

<style lang="scss" scoped>
.wrap {
  display: grid;
  place-items: center;
  height: 100%;
}
</style>
