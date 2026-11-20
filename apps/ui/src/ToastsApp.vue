<script setup lang="ts">
// Finestra delle notifiche sopra lo stream (index.html#toasts): trasparente e sempre in primo
// piano, perché la finestra del video copre la pagina di Falco. Riceve le notifiche dalla
// finestra principale e le rimanda i clic.
import { nextTick, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { CircleAlert, X } from 'lucide-vue-next';
import UiAvatar from '@/components/UiAvatar.vue';
import UiButton from '@/components/UiButton.vue';

export interface OverlayItem {
  kind: 'knock' | 'request' | 'play' | 'invite' | 'error' | 'msg';
  id: string;
  name?: string;
  text: string;
  avatar?: string | null;
  accent?: string;
  /** Pulsanti: [rifiuta/chiudi, accetta/azione]. */
  no?: string;
  yes?: string;
}

const { t } = useI18n();
const items = ref<OverlayItem[]>([]);
const stack = ref<HTMLElement | null>(null);

async function send(item: OverlayItem, ok: boolean): Promise<void> {
  const { emitTo } = await import('@tauri-apps/api/event');
  await emitTo('main', 'toasts://action', { kind: item.kind, id: item.id, ok });
}

// Altezza reale delle notifiche: la finestra è grande solo quanto serve
async function report(): Promise<void> {
  await nextTick();
  const { emitTo } = await import('@tauri-apps/api/event');
  await emitTo('main', 'toasts://height', Math.ceil(stack.value?.getBoundingClientRect().height ?? 0));
}

onMounted(async () => {
  document.body.classList.add('pill');
  const { listen } = await import('@tauri-apps/api/event');
  await listen<OverlayItem[]>('toasts://state', (e) => {
    items.value = e.payload;
    void report();
  });
  const { emitTo } = await import('@tauri-apps/api/event');
  await emitTo('main', 'toasts://ready', null);
});
</script>

<template>
  <div ref="stack" class="stack" aria-live="polite">
    <div v-for="i in items" :key="`${i.kind}${i.id}`" class="toast" :class="i.kind" role="alert">
      <CircleAlert v-if="i.kind === 'error'" class="icon" :size="18" :stroke-width="1.75" aria-hidden="true" />
      <UiAvatar v-else-if="i.name" :name="i.name" :src="i.avatar" :accent="i.accent" :size="32" />
      <span class="text">{{ i.text }}</span>
      <div class="acts">
        <UiButton v-if="i.yes" variant="primary" :size="32" @click="send(i, true)">{{ i.yes }}</UiButton>
        <UiButton v-if="i.no" variant="ghost" :size="32" @click="send(i, false)">{{ i.no }}</UiButton>
        <button v-if="!i.no" type="button" class="close" :aria-label="t('app.close')" @click="send(i, false)">
          <X :size="16" :stroke-width="1.75" aria-hidden="true" />
        </button>
      </div>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.stack {
  display: grid;
  gap: $sp-2;
  padding: $sp-2;

  .toast {
    @include floating-card;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: $sp-3;
    padding: $sp-3 $sp-3 $sp-3 $sp-4;

    &.error .icon {
      color: $danger;
    }

    .text {
      flex: 1;
      min-width: 0;
      font-size: $fs-small;
      line-height: $lh-small;
    }

    .acts {
      display: flex;
      gap: $sp-1;
      margin-left: auto;
    }

    .close {
      @include reset-button;
      @include focus-ring;
      display: grid;
      place-items: center;
      width: 28px;
      height: 28px;
      border-radius: $r-field;
      color: $text-2;

      &:hover {
        background: $bg-3;
        color: $text-1;
      }
    }
  }
}
</style>
