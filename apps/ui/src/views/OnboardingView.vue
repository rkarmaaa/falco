<script setup lang="ts">
// Onboarding (§13): benvenuto, nome e avatar. La rete (Tailscale sulla rete Falco) si prepara da sola,
// senza account: al massimo una conferma di Windows o macOS per l'installazione.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import logo from '@/assets/falco-256.png';
import UiButton from '@/components/UiButton.vue';
import UiField from '@/components/UiField.vue';
import AvatarPicker from '@/components/AvatarPicker.vue';
import { createProfile, setAvatar } from '@/stores/social';
import { core } from '@/stores/core';
import { isRpcError, rpc } from '@/ipc';
import { attempt } from '@/stores/errors';

const { t } = useI18n();
const step = ref<'welcome' | 'profile'>('welcome');
const name = ref('');
const file = ref<File | null>(null);
const preview = ref<string | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);
const valid = computed(() => [...name.value.trim()].length >= 2 && [...name.value.trim()].length <= 24);
const tailscale = computed(() => core.snapshot?.network.tailscale ?? 'missing');

function pick(f: File): void {
  if (preview.value) URL.revokeObjectURL(preview.value);
  file.value = f;
  preview.value = URL.createObjectURL(f);
}

function unpick(): void {
  if (preview.value) URL.revokeObjectURL(preview.value);
  file.value = null;
  preview.value = null;
}

const phase = ref<'idle' | 'network' | 'profile'>('idle');

async function finish(): Promise<void> {
  busy.value = true;
  error.value = null;
  try {
    // Prima la rete: dopo il profilo questa schermata sparisce. Un errore qui non blocca:
    // resta un avviso e si riprova da Impostazioni → Rete.
    if (tailscale.value === 'missing') {
      phase.value = 'network';
      await attempt(t('onboarding.networkError'), () => rpc('tailscale.install'));
    }
    phase.value = 'profile';
    await createProfile(name.value.trim());
    if (file.value) await setAvatar(file.value);
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  } finally {
    busy.value = false;
    phase.value = 'idle';
  }
}
</script>

<template>
  <section class="onboarding">
    <div v-if="step === 'welcome'" class="card">
      <img class="logo" :src="logo" width="128" height="128" alt="Falco" />
      <h1 class="display">{{ $t('onboarding.welcome') }}</h1>
      <p class="tagline">{{ $t('app.tagline') }}</p>
      <p class="body">{{ $t('onboarding.intro') }}</p>
      <UiButton variant="primary" :size="48" @click="step = 'profile'">{{ $t('onboarding.continue') }}</UiButton>
    </div>

    <form v-else class="card" @submit.prevent="valid && finish()">
      <h1 class="title">{{ $t('onboarding.profileTitle') }}</h1>
      <AvatarPicker :name="name" :src="preview" stacked @pick="pick" @clear="unpick" />
      <UiField v-model="name" :label="$t('onboarding.name')" :placeholder="$t('onboarding.namePlaceholder')" :maxlength="24" />
      <p class="hint">{{ $t('onboarding.nameHint') }}</p>
      <p class="ts">{{ $t(`onboarding.network.${tailscale}`) }}</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <UiButton variant="primary" :size="48" type="submit" :disabled="!valid || busy">
        {{ phase === 'network' ? $t('onboarding.preparingNetwork') : phase === 'profile' ? $t('onboarding.creating') : $t('onboarding.done') }}
      </UiButton>
    </form>
  </section>
</template>

<style lang="scss" scoped>
.onboarding {
  display: grid;
  place-items: center;
  min-height: 100%;
  padding: $sp-8;

  .card {
    display: grid;
    justify-items: center;
    gap: $sp-4;
    width: min(420px, 100%);
    text-align: center;

    .logo {
      border-radius: 22%;
      box-shadow: 0 12px 40px $accent-glow;
    }

    .display {
      font-size: $fs-display;
      line-height: 38px;
      font-weight: 700;
      letter-spacing: -0.02em;
    }

    .title {
      font-size: $fs-h1;
      line-height: 30px;
      font-weight: 650;
    }

    .tagline {
      color: $accent-fg;
      font-weight: 600;
      margin-top: -$sp-2;
    }

    .body,
    .hint {
      color: $text-2;
    }

    .hint {
      justify-self: stretch;
      text-align: left;
      font-size: $fs-small;
      margin-top: -$sp-2;
    }

    :deep(.field) {
      justify-self: stretch;
      text-align: left;
    }

    .ts {
      @include message;
    }

    .error {
      @include message($danger);
    }
  }
}
</style>
