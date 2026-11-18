<script setup lang="ts">
// Collegamento diretto a un host per indirizzo (provvisorio M1, sostituito dagli inviti in M3)
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Link2, Play } from 'lucide-vue-next';
import UiButton from './UiButton.vue';
import UiField from './UiField.vue';
import { core, join, pairWith } from '@/stores/core';
import { isRpcError } from '@/ipc';

const { t } = useI18n();
const address = ref('');
const error = ref<string | null>(null);
const busy = computed(() => core.session.phase !== 'idle');

async function run(action: () => Promise<void>): Promise<void> {
  error.value = null;
  try {
    await action();
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  }
}
</script>

<template>
  <section class="connect">
    <header class="head">
      <h2 class="title">{{ $t('connect.title') }}</h2>
      <span class="badge">{{ $t('connect.provisional') }}</span>
    </header>
    <p class="hint">{{ $t('connect.hint') }}</p>
    <div class="row">
      <UiField v-model="address" :label="$t('connect.address')" placeholder="100.64.0.2" />
      <UiButton variant="secondary" :disabled="!address || busy" @click="run(() => pairWith(address))">
        <Link2 :size="18" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('connect.pair') }}
      </UiButton>
      <UiButton variant="primary" :disabled="!address || busy" @click="run(() => join(address))">
        <Play :size="18" :stroke-width="1.75" aria-hidden="true" />
        {{ $t('connect.join') }}
      </UiButton>
    </div>

    <p v-if="core.pairing" class="status" :class="core.pairing.result" aria-live="polite">
      <template v-if="core.pairing.result === 'pending'">
        {{ $t('connect.pinPrompt') }} <strong class="pin">{{ core.pairing.pin }}</strong>
      </template>
      <template v-else-if="core.pairing.result === 'ok'">{{ $t('connect.paired', { host: core.pairing.host }) }}</template>
      <template v-else>{{ $t('connect.pairFailed') }}</template>
    </p>
    <p v-if="core.session.phase !== 'idle'" class="status pending" aria-live="polite">
      {{ $t(`connect.phase.${core.session.phase}`) }}
    </p>
    <p v-else-if="core.lastSession" class="status ok">
      {{ $t('connect.ended', { minutes: Math.max(1, Math.round(core.lastSession.durationS / 60)) }) }}
    </p>
    <p v-if="error" class="status failed" role="alert">{{ error }}</p>
  </section>
</template>

<style lang="scss" scoped>
.connect {
  @include surface(2);
  display: grid;
  gap: $sp-3;
  margin-top: $sp-6;
  padding: $sp-5;
  border-radius: $r-card;

  .head {
    display: flex;
    align-items: center;
    gap: $sp-3;

    .title {
      font-size: $fs-h2;
      line-height: 24px;
      font-weight: 600;
    }

    .badge {
      @include label;
      padding: 2px $sp-2;
      border: 1px solid $line;
      border-radius: $r-pill;
    }
  }

  .hint {
    color: $text-2;
    font-size: $fs-small;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: end;
    gap: $sp-3;
  }

  .status {
    font-size: $fs-small;
    color: $text-2;

    &.ok {
      color: $ok;
    }

    &.failed {
      color: $danger;
    }

    .pin {
      font-size: $fs-h2;
      letter-spacing: 0.2em;
      color: $text-1;
      font-variant-numeric: tabular-nums;
    }
  }
}
</style>
