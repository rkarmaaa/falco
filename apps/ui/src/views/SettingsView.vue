<script setup lang="ts">
// Impostazioni: la UI è solo un editor di config.toml (§15). Salvataggio automatico.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { RefreshCw } from 'lucide-vue-next';
import UiSegmented from '@/components/UiSegmented.vue';
import UiButton from '@/components/UiButton.vue';
import DiagnosticRow from '@/components/DiagnosticRow.vue';
import UiAvatar from '@/components/UiAvatar.vue';
import UiField from '@/components/UiField.vue';
import ConnectCard from '@/components/ConnectCard.vue';
import SettingsForm from '@/components/SettingsForm.vue';
import { core, setConfig, setStatus } from '@/stores/core';
import { rename, setAvatar, social } from '@/stores/social';
import { isRpcError, rpc, type DoctorCheck, type Status } from '@/ipc';

type Tab = 'profile' | 'streaming' | 'network' | 'host' | 'permissions' | 'advanced' | 'about';
const tabs: Tab[] = ['profile', 'streaming', 'network', 'host', 'permissions', 'advanced', 'about'];
const soon: Partial<Record<Tab, string>> = {};

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

const name = ref(social.profile.name ?? '');
const profileError = ref<string | null>(null);
async function saveName(): Promise<void> {
  profileError.value = null;
  try {
    await rename(name.value);
    saved.value = true;
    setTimeout(() => (saved.value = false), 1500);
  } catch (e) {
    profileError.value = isRpcError(e) ? e.message : t('connect.genericError');
  }
}
async function pickAvatar(e: Event): Promise<void> {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  try {
    await setAvatar(f);
  } catch (err) {
    profileError.value = isRpcError(err) ? err.message : t('connect.genericError');
  }
}

const oauthId = ref('');
const oauthSecret = ref('');
const oauthMsg = ref<{ ok: boolean; text: string } | null>(null);
async function saveOAuth(): Promise<void> {
  try {
    await rpc('tailscale.setOAuth', { clientId: oauthId.value.trim(), clientSecret: oauthSecret.value.trim() });
    oauthMsg.value = { ok: true, text: t('settings.oauth.saved') };
    oauthSecret.value = '';
  } catch (e) {
    oauthMsg.value = { ok: false, text: isRpcError(e) ? e.message : t('connect.genericError') };
  }
}

function flashSaved(): void {
  saved.value = true;
  setTimeout(() => (saved.value = false), 1500);
}

async function exportConfig(): Promise<void> {
  const { text } = await rpc<{ text: string }>('config.export');
  const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' }));
  const a = Object.assign(document.createElement('a'), { href: url, download: 'falco-config.toml' });
  a.click();
  URL.revokeObjectURL(url);
}

const importMsg = ref<{ ok: boolean; text: string } | null>(null);
async function importConfig(e: Event): Promise<void> {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  try {
    await rpc('config.import', { text: await f.text() });
    importMsg.value = { ok: true, text: t('settings.imported') };
  } catch (err) {
    importMsg.value = { ok: false, text: isRpcError(err) ? err.message : t('connect.genericError') };
  }
}

async function headscaleLogin(): Promise<void> {
  try {
    await rpc('net.loginHeadscale');
    importMsg.value = { ok: true, text: t('settings.headscaleStarted') };
  } catch (err) {
    importMsg.value = { ok: false, text: isRpcError(err) ? err.message : t('connect.genericError') };
  }
}

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
        <div class="me">
          <label class="avatar-pick">
            <UiAvatar :name="social.profile.name ?? '?'" :src="social.profile.avatarUrl" :accent="social.profile.accent" :size="56" />
            <span class="link">{{ $t('settings.changeAvatar') }}</span>
            <input class="sr-only" type="file" accept="image/*" @change="pickAvatar" />
          </label>
          <form class="rename" @submit.prevent="saveName">
            <UiField v-model="name" :label="$t('onboarding.name')" :maxlength="24" />
            <UiButton variant="secondary" type="submit">{{ $t('settings.save') }}</UiButton>
          </form>
          <span class="tag">{{ social.profile.tag }}</span>
        </div>
        <p v-if="profileError" class="error" role="alert">{{ profileError }}</p>
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
          <DiagnosticRow v-for="(c, i) in checks" :key="c.id + i" :check="c" />
        </ul>
        <h2 class="subtitle">{{ $t('settings.networkParams') }}</h2>
        <SettingsForm :sections="['network']" @saved="flashSaved" />
        <UiButton
          v-if="core.config?.network.provider === 'headscale'"
          variant="secondary"
          :size="32"
          @click="headscaleLogin"
        >
          {{ $t('settings.headscaleLogin') }}
        </UiButton>
      </template>

      <template v-else-if="tab === 'streaming'">
        <SettingsForm :sections="['stream']" @saved="flashSaved" />
      </template>

      <template v-else-if="tab === 'host'">
        <h2 class="subtitle">{{ $t('settings.oauth.title') }}</h2>
        <p class="muted">{{ core.snapshot?.oauthConfigured ? $t('settings.oauth.configured') : $t('settings.oauth.intro') }}</p>
        <form class="oauth" @submit.prevent="saveOAuth">
          <UiField v-model="oauthId" :label="$t('settings.oauth.clientId')" />
          <label class="field">
            <span class="label">{{ $t('settings.oauth.secret') }}</span>
            <input v-model="oauthSecret" class="secret" type="password" autocomplete="off" />
          </label>
          <UiButton variant="secondary" type="submit" :disabled="!oauthId || !oauthSecret">{{ $t('settings.save') }}</UiButton>
        </form>
        <p v-if="oauthMsg" :class="oauthMsg.ok ? 'ok' : 'error'" role="status">{{ oauthMsg.text }}</p>
        <h2 class="subtitle">{{ $t('settings.hostParams') }}</h2>
        <SettingsForm :sections="['host']" @saved="flashSaved" />
      </template>

      <template v-else-if="tab === 'permissions'">
        <p class="muted">{{ $t('settings.permissionsIntro') }}</p>
        <SettingsForm :sections="['host']" @saved="flashSaved" />
      </template>

      <template v-else-if="tab === 'advanced'">
        <SettingsForm :sections="['general', 'performance', 'advanced']" @saved="flashSaved" />
        <div class="io">
          <UiButton variant="secondary" :size="32" @click="exportConfig">{{ $t('settings.export') }}</UiButton>
          <label class="import">
            <span>{{ $t('settings.import') }}</span>
            <input class="sr-only" type="file" accept=".toml,text/plain" @change="importConfig" />
          </label>
        </div>
        <p v-if="importMsg" :class="importMsg.ok ? 'ok' : 'error'" role="status">{{ importMsg.text }}</p>
        <ConnectCard />
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
        border-bottom-color: $accent-fg;
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

    .io {
      display: flex;
      gap: $sp-2;

      .import {
        @include surface(2);
        display: inline-flex;
        align-items: center;
        height: 32px;
        padding: 0 $sp-3;
        border-radius: $r-button;
        font-size: $fs-small;
        font-weight: 600;
        cursor: pointer;

        &:focus-within {
          border-color: $accent-fg;
        }
      }
    }

    .error {
      color: $danger;
      font-size: $fs-small;
    }

    .ok {
      color: $ok;
      font-size: $fs-small;
    }

    .subtitle {
      font-size: $fs-h2;
      line-height: 24px;
      font-weight: 600;
    }

    .me {
      display: flex;
      align-items: flex-end;
      gap: $sp-5;

      .avatar-pick {
        display: grid;
        justify-items: center;
        gap: $sp-1;
        cursor: pointer;

        .link {
          font-size: $fs-small;
          color: $text-2;
        }

        &:focus-within .link {
          color: $accent-fg;
        }
      }

      .rename {
        display: flex;
        align-items: flex-end;
        gap: $sp-2;
      }

      .tag {
        margin-bottom: $sp-2;
        color: $text-3;
        font-variant-numeric: tabular-nums;
      }
    }

    .oauth {
      display: grid;
      grid-template-columns: 1fr 1fr auto;
      align-items: end;
      gap: $sp-3;

      .field {
        display: grid;
        gap: $sp-1;

        .label {
          @include label;
        }
      }

      .secret {
        @include surface(2);
        @include focus-ring;
        height: $control-h;
        padding: 0 $sp-3;
        border-radius: $r-field;
        color: $text-1;
        font: inherit;
      }
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
