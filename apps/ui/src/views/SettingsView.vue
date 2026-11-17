<script setup lang="ts">
// Impostazioni: la UI è solo un editor di config.toml (§15). Salvataggio automatico.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { RefreshCw } from 'lucide-vue-next';
import UiSegmented from '@/components/UiSegmented.vue';
import UiButton from '@/components/UiButton.vue';
import DiagnosticRow from '@/components/DiagnosticRow.vue';
import { core, setConfig, setStatus } from '@/stores/core';
import { rpc, type DoctorCheck, type Status } from '@/ipc';

type Tab = 'profile' | 'streaming' | 'network' | 'host' | 'permissions' | 'advanced' | 'about';
const tabs: Tab[] = ['profile', 'streaming', 'network', 'host', 'permissions', 'advanced', 'about'];
const soon: Partial<Record<Tab, string>> = { streaming: 'M1', host: 'M1', permissions: 'M4', advanced: 'M5' };

const { t } = useI18n();
const tab = ref<Tab>('profile');
const saved = ref(false);

async function save(key: string, value: unknown): Promise<void> {
  await setConfig(key, value);
  saved.value = true;
  setTimeout(() => (saved.value = false), 1500);
}

const language = computed({
  get: () => core.config?.general.language ?? 'it',
  set: (v) => void save('general.language', v),
});
const theme = computed({
  get: () => core.config?.general.theme ?? 'system',
  set: (v) => void save('general.theme', v),
});
const status = computed<Status>({
  get: () => core.snapshot?.status ?? 'online',
  set: (v) => void setStatus(v),
});

const checks = ref<DoctorCheck[]>([]);
const running = ref(false);
async function runDoctor(): Promise<void> {
  running.value = true;
  try {
    checks.value = (await rpc<{ checks: DoctorCheck[] }>('net.runDoctor')).checks;
  } finally {
    running.value = false;
  }
}

function openTab(next: Tab): void {
  tab.value = next;
  if (next === 'network' && checks.value.length === 0) void runDoctor();
}
</script>

<template>
  <section class="settings">
    <header class="head">
      <h1 class="title">{{ $t('settings.title') }}</h1>
      <span class="saved" :class="{ show: saved }" aria-live="polite">{{ saved ? $t('settings.saved') : '' }}</span>
    </header>

    <div class="tabs" role="tablist">
      <button
        v-for="id in tabs"
        :key="id"
        type="button"
        role="tab"
        class="tab"
        :class="{ active: tab === id }"
        :aria-selected="tab === id"
        @click="openTab(id)"
      >
        {{ $t(`settings.tabs.${id}`) }}
      </button>
    </div>

    <div class="panel" role="tabpanel">
      <template v-if="tab === 'profile'">
        <div class="field">
          <span class="label">{{ $t('settings.status') }}</span>
          <UiSegmented
            v-model="status"
            :label="t('settings.status')"
            :options="(['online', 'busy', 'invisible'] as const).map((v) => ({ value: v, label: t(`status.${v}`) }))"
          />
        </div>
        <div class="field">
          <span class="label">{{ $t('settings.language') }}</span>
          <UiSegmented
            v-model="language"
            :label="t('settings.language')"
            :options="[
              { value: 'it', label: 'Italiano' },
              { value: 'en', label: 'English' },
            ]"
          />
        </div>
        <div class="field">
          <span class="label">{{ $t('settings.theme') }}</span>
          <UiSegmented
            v-model="theme"
            :label="t('settings.theme')"
            :options="(['system', 'dark', 'light'] as const).map((v) => ({ value: v, label: t(`settings.themes.${v}`) }))"
          />
        </div>
      </template>

      <template v-else-if="tab === 'network'">
        <div class="doctor-head">
          <h2 class="subtitle">{{ $t('doctor.title') }}</h2>
          <UiButton variant="secondary" :size="32" :disabled="running" @click="runDoctor">
            <RefreshCw :size="16" :stroke-width="1.75" aria-hidden="true" />
            {{ running ? $t('doctor.running') : $t('doctor.rerun') }}
          </UiButton>
        </div>
        <ul class="doctor">
          <DiagnosticRow v-for="c in checks" :key="c.id" :check="c" />
        </ul>
      </template>

      <template v-else-if="tab === 'about'">
        <p>{{ $t('settings.version', { version: core.snapshot?.version ?? '–', milestone: core.snapshot?.milestone ?? '–' }) }}</p>
        <p class="muted">{{ $t('settings.license') }}</p>
      </template>

      <p v-else class="muted">{{ $t('settings.soon', { milestone: soon[tab] ?? 'M6' }) }}</p>
    </div>
  </section>
</template>

<style lang="scss" scoped>
.settings {
  display: grid;
  gap: $sp-5;
  max-width: 760px;

  .head {
    display: flex;
    align-items: baseline;
    gap: $sp-4;

    .title {
      font-size: $fs-h1;
      line-height: 30px;
      font-weight: 650;
    }

    .saved {
      font-size: $fs-small;
      color: $ok;
      opacity: 0;
      transition: opacity $t-fast $ease;

      &.show {
        opacity: 1;
      }
    }
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: $sp-1;
    border-bottom: 1px solid $line;

    .tab {
      @include reset-button;
      @include focus-ring;
      height: $control-h;
      padding: 0 $sp-3;
      color: $text-2;
      font-weight: 600;
      border-bottom: 2px solid transparent;
      margin-bottom: -1px;
      transition: color $t-fast $ease;

      &:hover {
        color: $text-1;
      }

      &.active {
        color: $text-1;
        border-bottom-color: $ember-500;
      }
    }
  }

  .panel {
    display: grid;
    gap: $sp-5;

    .field {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: $sp-4;

      .label {
        font-weight: 600;
      }
    }

    .muted {
      color: $text-2;
    }

    .doctor-head {
      display: flex;
      align-items: center;
      justify-content: space-between;

      .subtitle {
        font-size: $fs-h2;
        line-height: 24px;
        font-weight: 600;
      }
    }

    .doctor {
      @include surface(2);
      margin: 0;
      padding: 0;
      list-style: none;
      border-radius: $r-card;
    }
  }
}
</style>
