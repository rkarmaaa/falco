<script setup lang="ts">
// Palco dello stream: qui dentro compare la finestra di Moonlight. Finché non c'è,
// mostra a che punto è la connessione; nel browser simula un flusso.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useResizeObserver } from '@vueuse/core';
import { ExternalLink, MousePointerClick } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import { core } from '@/stores/core';
import { social } from '@/stores/social';
import { attach, conceal, place, stage } from '@/stores/stage';
import { isTauri } from '@/ipc';

const { t } = useI18n();
const el = ref<HTMLElement | null>(null);
const friend = computed(() => {
  const id = core.session.info?.friendId ?? core.session.friendId;
  return social.friends.find((f) => f.id === id);
});
const steps = computed(() => {
  const p = core.session.phase;
  const link = core.session.linked ? [{ id: 'link', done: p !== 'linking' }] : [];
  return [
    ...link,
    { id: 'knock', done: p !== 'linking' && p !== 'starting' },
    { id: 'connect', done: p === 'connected' },
    { id: 'video', done: stage.embedded },
  ];
});

// Il video sta sempre dentro il palco: rettangolo 16:9 centrato, mai più largo o più alto
function rect() {
  const r = el.value?.getBoundingClientRect();
  if (!r || r.width <= 0 || r.height <= 0) return null;
  const w = Math.min(r.width, (r.height * 16) / 9);
  const h = Math.min(r.height, (r.width * 9) / 16);
  return { x: r.left + (r.width - w) / 2, y: r.top + (r.height - h) / 2, w, h };
}

useResizeObserver(el, () => place(rect()));
onMounted(() => {
  window.addEventListener('resize', onResize);
  place(rect());
});
onBeforeUnmount(() => {
  window.removeEventListener('resize', onResize);
  // Altra sezione: lo stream resta attivo ma nascosto
  conceal();
});
function onResize(): void {
  place(rect());
}

// Appena c'è il processo di Moonlight si cerca la sua finestra
function tryAttach(): void {
  const pid = core.session.info?.pid;
  const r = rect();
  if (pid && r) void attach(pid, r);
}
watch(() => core.session.info?.pid, tryAttach);
onMounted(tryAttach);
</script>

<template>
  <section ref="el" class="stage" :class="{ embedded: stage.embedded, fullscreen: stage.fullscreen }" :aria-label="t('session.stage', { name: core.session.host })">
    <!-- Simulazione nel browser -->
    <div v-if="stage.embedded && !isTauri()" class="mock" aria-hidden="true">
      <div class="sky" />
      <span>{{ t('session.mock') }}</span>
    </div>

    <div v-else-if="stage.external" class="center">
      <ExternalLink :size="28" :stroke-width="1.6" aria-hidden="true" />
      <h2>{{ t('session.externalTitle') }}</h2>
      <p>{{ t('session.externalBody') }}</p>
    </div>

    <div v-else-if="!stage.embedded" class="center">
      <div class="halo">
        <UiAvatar :name="core.session.host ?? '?'" :src="friend?.avatarUrl" :accent="friend?.accent" :size="96" />
      </div>
      <h2>{{ t('session.joining', { name: core.session.host }) }}</h2>
      <ol class="steps">
        <li v-for="s in steps" :key="s.id" :class="{ done: s.done }">
          <span class="dot" aria-hidden="true" />
          {{ t(`session.steps.${s.id}`) }}
        </li>
      </ol>
      <p class="tip"><MousePointerClick :size="14" :stroke-width="1.9" aria-hidden="true" />{{ t('session.tip') }}</p>
    </div>
  </section>
</template>

<style lang="scss" scoped>
.stage {
  position: relative;
  display: grid;
  place-items: center;
  height: 100%;
  overflow: hidden;
  background: #000;
  border-radius: $r-card;
  box-shadow: 0 0 0 1px $line;

  &.fullscreen {
    border-radius: 0;
    box-shadow: none;
  }

  .center {
    display: grid;
    justify-items: center;
    gap: $sp-3;
    max-width: 420px;
    padding: $sp-6;
    text-align: center;
    color: $text-2;

    h2 {
      color: $text-1;
      font-size: $fs-h2;
      font-weight: 650;
    }

    .halo {
      padding: 6px;
      margin-bottom: $sp-2;
      border-radius: $r-pill;
      background: conic-gradient(from 0deg, $accent-400, $live, $accent-400);
      animation: spin 2.6s linear infinite;

      :deep(.avatar) {
        animation: spin 2.6s linear infinite reverse;
      }
    }

    .steps {
      display: grid;
      gap: $sp-2;
      margin: $sp-2 0 0;
      padding: 0;
      list-style: none;
      text-align: left;
      font-size: $fs-small;

      li {
        display: flex;
        align-items: center;
        gap: $sp-2;
        color: $text-3;

        .dot {
          width: 8px;
          height: 8px;
          border-radius: $r-pill;
          box-shadow: inset 0 0 0 1.5px $text-3;
        }

        &.done {
          color: $text-1;

          .dot {
            background: $live;
            box-shadow: 0 0 8px $live-glow;
          }
        }
      }
    }

    .tip {
      display: flex;
      align-items: center;
      gap: 6px;
      margin-top: $sp-3;
      font-size: 12px;
      color: $text-3;
    }
  }

  .mock {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: end start;
    padding: $sp-4;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;

    .sky {
      position: absolute;
      inset: 0;
      background:
        radial-gradient(60% 80% at 30% 20%, rgba(76, 95, 214, 0.65), transparent 60%),
        radial-gradient(50% 60% at 80% 80%, rgba(70, 224, 169, 0.35), transparent 60%),
        linear-gradient(160deg, #0d1030, #05060c);
      animation: drift 12s ease-in-out infinite alternate;
    }

    span {
      position: relative;
    }
  }
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes drift {
  to {
    filter: hue-rotate(40deg);
    transform: scale(1.08);
  }
}
</style>
