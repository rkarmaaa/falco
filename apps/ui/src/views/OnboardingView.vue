<script setup lang="ts">
// Onboarding (§13): benvenuto, nome e avatar, controllo di Tailscale. Obiettivo < 3 minuti.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ImagePlus } from 'lucide-vue-next';
import logo from '@/assets/falco-256.png';
import UiButton from '@/components/UiButton.vue';
import UiField from '@/components/UiField.vue';
import UiAvatar from '@/components/UiAvatar.vue';
import { createProfile, setAvatar } from '@/stores/social';
import { core } from '@/stores/core';
import { isRpcError } from '@/ipc';

const { t } = useI18n();
const step = ref<'welcome' | 'profile'>('welcome');
const name = ref('');
const file = ref<File | null>(null);
const preview = ref<string | null>(null);
const error = ref<string | null>(null);
const busy = ref(false);
const valid = computed(() => [...name.value.trim()].length >= 2 && [...name.value.trim()].length <= 24);
const tailscale = computed(() => core.snapshot?.network.tailscale ?? 'missing');

function pick(e: Event): void {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (!f) return;
  file.value = f;
  preview.value = URL.createObjectURL(f);
}

async function finish(): Promise<void> {
  busy.value = true;
  error.value = null;
  try {
    await createProfile(name.value.trim());
    if (file.value) await setAvatar(file.value);
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  } finally {
    busy.value = false;
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
      <label class="avatar-pick">
        <UiAvatar :name="name || '?'" :src="preview" :size="96" />
        <span class="pick"><ImagePlus :size="16" :stroke-width="1.75" aria-hidden="true" /> {{ $t('onboarding.avatar') }}</span>
        <input class="sr-only" type="file" accept="image/*" @change="pick" />
      </label>
      <UiField v-model="name" :label="$t('onboarding.name')" :placeholder="$t('onboarding.namePlaceholder')" :maxlength="24" />
      <p class="hint">{{ $t('onboarding.nameHint') }}</p>
      <p class="ts" :class="tailscale">{{ $t(`onboarding.tailscale.${tailscale}`) }}</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <UiButton variant="primary" :size="48" type="submit" :disabled="!valid || busy">{{ $t('onboarding.done') }}</UiButton>
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

    .avatar-pick {
      display: grid;
      justify-items: center;
      gap: $sp-2;
      cursor: pointer;

      .pick {
        display: inline-flex;
        align-items: center;
        gap: $sp-1;
        color: $text-2;
        font-size: $fs-small;
      }

      &:focus-within .pick {
        color: $accent-fg;
      }
    }

    .ts {
      font-size: $fs-small;
      color: $busy;

      &.running {
        color: $ok;
      }
    }

    .error {
      color: $danger;
      font-size: $fs-small;
    }
  }
}
</style>
