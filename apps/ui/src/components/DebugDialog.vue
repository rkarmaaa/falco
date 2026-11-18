<script setup lang="ts">
// Schermata «Debug»: tutto ciò che serve per capire un errore, a sezioni copiabili.
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Copy } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiButton from './UiButton.vue';
import { closeDebug, describe, errors, type AppError } from '@/stores/errors';
import { core } from '@/stores/core';
import { isRpcError, isTauri, rpc } from '@/ipc';
import { copyText } from '@/utils/clipboard';

const props = defineProps<{ error: AppError }>();
const { t, locale } = useI18n();
const report = ref<Record<string, unknown> | null>(null);
const reportError = ref<string | null>(null);
const copied = ref<string | null>(null);

onMounted(async () => {
  try {
    report.value = await rpc<Record<string, unknown>>('debug.report');
  } catch (e) {
    reportError.value = isRpcError(e) ? `${e.message} (${e.code})` : String(e);
  }
});

const json = (v: unknown) => JSON.stringify(v, null, 2);

const sections = computed(() => {
  const r = report.value ?? {};
  const logs = (r.logs ?? {}) as { core?: string[]; sunshine?: string[] };
  const list: { id: string; title: string; text: string }[] = [
    { id: 'error', title: t('debug.sections.error'), text: describe(props.error) + (props.error.technical.stack ? `\n\n${props.error.technical.stack}` : '') },
    {
      id: 'ui',
      title: t('debug.sections.ui'),
      text: json({
        shell: isTauri() ? 'tauri' : 'browser',
        userAgent: navigator.userAgent,
        language: locale.value,
        theme: document.documentElement.dataset.theme,
        window: `${window.innerWidth}×${window.innerHeight} @${window.devicePixelRatio}`,
        coreConnected: core.connected,
        host: core.host,
        session: core.session,
        uiErrors: errors.history.slice(0, 10).map((e) => ({ at: e.at, source: e.technical.source, code: e.technical.code, message: e.technical.message })),
      }),
    },
  ];
  if (reportError.value) {
    list.push({ id: 'core', title: t('debug.sections.core'), text: t('debug.coreUnavailable', { why: reportError.value }) });
  } else if (report.value) {
    list.push(
      { id: 'app', title: t('debug.sections.app'), text: json({ app: r.app, state: r.state }) },
      { id: 'network', title: t('debug.sections.network'), text: json(r.network) },
      { id: 'engines', title: t('debug.sections.engines'), text: json(r.engines) },
      { id: 'session', title: t('debug.sections.session'), text: json(r.lastSession ?? null) },
      { id: 'coreErrors', title: t('debug.sections.coreErrors'), text: json(r.errors) },
      { id: 'config', title: t('debug.sections.config'), text: json(r.config) },
      { id: 'coreLog', title: t('debug.sections.coreLog'), text: (logs.core ?? []).join('\n') || '—' },
      { id: 'sunshineLog', title: t('debug.sections.sunshineLog'), text: (logs.sunshine ?? []).join('\n') || '—' },
    );
  }
  return list;
});

const fullText = computed(() => sections.value.map((s) => `===== ${s.title} =====\n${s.text}`).join('\n\n'));

async function copy(id: string, text: string): Promise<void> {
  if (await copyText(text)) {
    copied.value = id;
    setTimeout(() => (copied.value = null), 2000);
  }
}
</script>

<template>
  <UiModal :title="$t('debug.title')" class="debug" @close="closeDebug">
    <p class="intro">{{ $t('debug.intro') }}</p>
    <p v-if="!report && !reportError" class="muted">{{ $t('debug.loading') }}</p>
    <section v-for="s in sections" :key="s.id" class="section">
      <header class="head">
        <h3 class="label">{{ s.title }}</h3>
        <button type="button" class="copy" @click="copy(s.id, s.text)">
          <Check v-if="copied === s.id" :size="14" :stroke-width="1.75" aria-hidden="true" />
          <Copy v-else :size="14" :stroke-width="1.75" aria-hidden="true" />
          {{ copied === s.id ? $t('errors.copied') : $t('errors.copy') }}
        </button>
      </header>
      <pre class="pre">{{ s.text }}</pre>
    </section>
    <template #actions>
      <UiButton variant="primary" @click="copy('all', fullText)">
        <Check v-if="copied === 'all'" :size="16" :stroke-width="1.75" aria-hidden="true" />
        <Copy v-else :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ copied === 'all' ? $t('errors.copied') : $t('debug.copyAll') }}
      </UiButton>
      <UiButton variant="secondary" @click="closeDebug">{{ $t('app.close') }}</UiButton>
    </template>
  </UiModal>
</template>

<style lang="scss" scoped>
.debug {
  width: min(820px, calc(100vw - #{$sp-8}));
}

.intro,
.muted {
  font-size: $fs-small;
  color: $text-2;
}

.section {
  display: grid;
  gap: $sp-1;

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;

    .label {
      @include label;
    }

    .copy {
      @include reset-button;
      @include focus-ring;
      display: inline-flex;
      align-items: center;
      gap: $sp-1;
      padding: 2px $sp-2;
      border-radius: $r-field;
      font-size: $fs-small;
      color: $text-2;

      &:hover {
        background: $bg-3;
        color: $text-1;
      }
    }
  }

  .pre {
    @include surface(2);
    margin: 0;
    max-height: 220px;
    overflow: auto;
    padding: $sp-2 $sp-3;
    border-radius: $r-field;
    font: 12px/1.45 ui-monospace, 'Cascadia Mono', Consolas, monospace;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
}
</style>
