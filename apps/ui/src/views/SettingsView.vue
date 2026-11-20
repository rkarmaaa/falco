<script setup lang="ts">
// Impostazioni: la UI è solo un editor di config.toml (§15). Salvataggio automatico.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Github, Globe, RefreshCw } from 'lucide-vue-next';
import UiSegmented from '@/components/UiSegmented.vue';
import UiButton from '@/components/UiButton.vue';
import DiagnosticRow from '@/components/DiagnosticRow.vue';
import AvatarPicker from '@/components/AvatarPicker.vue';
import UiField from '@/components/UiField.vue';
import ConnectCard from '@/components/ConnectCard.vue';
import SettingsForm from '@/components/SettingsForm.vue';
import UiPage from '@/components/UiPage.vue';
import { core, setConfig, setStatus } from '@/stores/core';
import { clearAvatar, rename, setAvatar, social } from '@/stores/social';
import { isRpcError, rpc, type DoctorCheck, type Status } from '@/ipc';

type Tab = 'profile' | 'network' | 'advanced' | 'about';
const tabs: Tab[] = ['profile', 'network', 'advanced', 'about'];
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
async function avatarAction(run: () => Promise<void>): Promise<void> {
  profileError.value = null;
  try {
    await run();
    flashSaved();
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

const update = ref<{ available: { version: string; notes: string } | null; current: string; enabled: boolean } | null>(null);
const updating = ref(false);
// Dal pulsante in alto: controlla e mostra il risultato in «Info»
const checking = ref(false);
async function checkFromHeader(): Promise<void> {
  checking.value = true;
  try {
    await checkUpdate();
    tab.value = 'about';
  } finally {
    checking.value = false;
  }
}

async function checkUpdate(): Promise<void> {
  update.value = await rpc('update.check');
}
async function installUpdate(): Promise<void> {
  updating.value = true;
  try {
    await rpc('update.install');
  } catch (err) {
    importMsg.value = { ok: false, text: isRpcError(err) ? err.message : t('connect.genericError') };
    updating.value = false;
  }
}

const joining = ref(false);
async function joinFalco(): Promise<void> {
  joining.value = true;
  try {
    await rpc('net.joinFalco');
    importMsg.value = { ok: true, text: t('settings.falcoJoined') };
  } catch (err) {
    importMsg.value = { ok: false, text: isRpcError(err) ? err.message : t('connect.genericError') };
  } finally {
    joining.value = false;
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
  <UiPage class="settings" :title="$t('settings.title')" narrow>
    <template #actions>
      <UiButton variant="ghost" :size="32" icon :title="$t('settings.website')" :aria-label="$t('settings.website')" @click="rpc('app.openWeb', { target: 'site' })">
        <Globe :size="17" :stroke-width="1.75" aria-hidden="true" />
      </UiButton>
      <UiButton variant="ghost" :size="32" icon title="GitHub" aria-label="GitHub" @click="rpc('app.openWeb', { target: 'github' })">
        <Github :size="17" :stroke-width="1.75" aria-hidden="true" />
      </UiButton>
      <UiButton
        variant="ghost"
        :size="32"
        icon
        :class="{ spinning: checking }"
        :title="$t('update.check')"
        :aria-label="$t('update.check')"
        @click="checkFromHeader"
      >
        <RefreshCw :size="17" :stroke-width="1.75" aria-hidden="true" />
      </UiButton>
    </template>
    <template #meta>
      <span class="saved" :class="{ show: saved }" aria-live="polite">{{ saved ? $t('settings.saved') : '' }}</span>
    </template>

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
        <AvatarPicker
          :name="social.profile.name ?? '?'"
          :src="social.profile.avatarUrl"
          :accent="social.profile.accent"
          @pick="(f) => avatarAction(() => setAvatar(f))"
          @clear="avatarAction(clearAvatar)"
        />
        <form class="rename" @submit.prevent="saveName">
          <UiField v-model="name" :label="$t('onboarding.name')" :maxlength="24" />
          <UiButton variant="secondary" type="submit">{{ $t('settings.save') }}</UiButton>
          <span class="tag">{{ social.profile.tag }}</span>
        </form>
        <p v-if="profileError" class="error" role="alert">{{ profileError }}</p>
        <div class="setting">
          <span class="label">{{ $t('settings.status') }}</span>
          <UiSegmented
            v-model="status"
            :label="t('settings.status')"
            :options="(['online', 'busy', 'invisible'] as const).map((v) => ({ value: v, label: t(`status.${v}`) }))"
          />
        </div>
        <div class="setting">
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
        <div v-if="core.config?.network.provider === 'falco'" class="io">
          <UiButton variant="secondary" :size="32" :disabled="joining" @click="joinFalco">
            {{ joining ? $t('settings.falcoJoining') : $t('settings.falcoJoin') }}
          </UiButton>
          <span class="muted">{{ $t('settings.falcoJoinHint') }}</span>
        </div>
        <p v-if="importMsg" :class="importMsg.ok ? 'ok' : 'error'" role="status">{{ importMsg.text }}</p>
        <details v-if="core.config?.network.provider === 'tailscale'" class="advanced-net">
          <summary class="subtitle">{{ $t('settings.oauth.title') }}</summary>
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
        </details>
        <UiButton
          v-if="core.config?.network.provider === 'headscale'"
          variant="secondary"
          :size="32"
          @click="headscaleLogin"
        >
          {{ $t('settings.headscaleLogin') }}
        </UiButton>
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
        <div class="io">
          <UiButton variant="secondary" :size="32" @click="checkUpdate">{{ $t('update.check') }}</UiButton>
          <UiButton v-if="update?.available" variant="primary" :size="32" :disabled="updating" @click="installUpdate">
            {{ updating ? $t('update.installing') : $t('update.installVersion', { version: update.available.version }) }}
          </UiButton>
        </div>
        <p v-if="update && !update.enabled" class="muted">{{ $t('update.disabled') }}</p>
        <p v-else-if="update && !update.available" class="ok">{{ $t('update.upToDate', { version: update.current }) }}</p>
        <p v-if="update?.available?.notes" class="muted">{{ update.available.notes }}</p>
        <p v-if="importMsg" :class="importMsg.ok ? 'ok' : 'error'" role="status">{{ importMsg.text }}</p>
      </template>

      <p v-else class="muted">{{ $t('settings.soon', { milestone: soon[tab] ?? 'M6' }) }}</p>
    </div>
  </UiPage>
</template>

<style lang="scss" scoped>
.settings {
  .saved {
    @include message($ok);
    opacity: 0;
    transition: opacity $t-fast $ease;

    &.show {
      opacity: 1;
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
    gap: $page-gap;

    .setting {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: $sp-4;

      .label {
        font-weight: $fw-strong;
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
      @include message($danger);
    }

    .ok {
      @include message($ok);
    }

    .subtitle {
      @include title-h2;
    }

    .rename {
      display: flex;
      align-items: flex-end;
      gap: $sp-2;

      .tag {
        align-self: center;
        margin-left: auto;
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

    }

    .advanced-net {
      display: grid;
      gap: $section-gap;

      summary {
        cursor: pointer;
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

.spinning :deep(svg) {
  animation: settings-spin 0.9s linear infinite;
}

@keyframes settings-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
