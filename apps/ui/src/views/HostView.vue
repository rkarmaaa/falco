<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Monitor } from 'lucide-vue-next';
import UiSwitch from '@/components/UiSwitch.vue';
import UiButton from '@/components/UiButton.vue';
import UiField from '@/components/UiField.vue';
import { core, hostApi, setHostAvailable } from '@/stores/core';
import type { PairedClient, PendingPairing } from '@/ipc';

const { t } = useI18n();
const available = computed({
  get: () => core.snapshot?.hostAvailable ?? false,
  set: (v: boolean) => void setHostAvailable(v),
});
const ready = computed(() => core.host.state === 'ready');

const pairings = ref<PendingPairing[]>([]);
const clients = ref<PairedClient[]>([]);
const pins = ref<Record<string, string>>({});
const names = ref<Record<string, string>>({});

async function load(): Promise<void> {
  if (!ready.value) return;
  [pairings.value, clients.value] = await Promise.all([hostApi.pairings(), hostApi.clients()]);
}

// Le richieste di associazione si aggiornano solo con la finestra aperta su Ospita
let timer: ReturnType<typeof setInterval> | undefined;
watch(
  ready,
  (r) => {
    clearInterval(timer);
    if (r) {
      void load();
      timer = setInterval(() => void load(), 3000);
    } else {
      pairings.value = [];
      clients.value = [];
    }
  },
  { immediate: true },
);
onBeforeUnmount(() => clearInterval(timer));

async function approve(p: PendingPairing): Promise<void> {
  await hostApi.approve(p.id, pins.value[p.id] ?? '', names.value[p.id] || p.name);
  await load();
}
</script>

<template>
  <section class="host">
    <h1 class="title">{{ $t('host.title') }}</h1>

    <div class="panel" :class="{ live: ready }">
      <UiSwitch v-model="available" :label="$t('host.available')" :hint="$t('host.availableHint')" />
      <p class="state" :class="core.host.state" aria-live="polite">
        <span class="dot" aria-hidden="true" />
        {{ $t(`host.state.${core.host.state}`) }}
        <template v-if="core.host.message"> · {{ core.host.message }}</template>
      </p>
      <UiButton v-if="ready" variant="danger" :size="32" class="end" @click="hostApi.endSession()">
        {{ $t('host.endSession') }}
      </UiButton>
    </div>

    <template v-if="ready">
      <h2 class="group">{{ $t('host.requests') }}</h2>
      <p v-if="pairings.length === 0" class="muted">{{ $t('host.noRequests') }}</p>
      <ul class="list">
        <li v-for="p in pairings" :key="p.id" class="item request">
          <div class="who">
            <span class="name">{{ p.name }}</span>
            <span class="meta">{{ p.address }}</span>
          </div>
          <UiField v-model="names[p.id]!" :label="$t('host.deviceName')" :placeholder="p.name" />
          <UiField v-model="pins[p.id]!" :label="$t('host.pin')" inputmode="numeric" :maxlength="4" placeholder="0000" />
          <UiButton variant="primary" :disabled="(pins[p.id] ?? '').length !== 4" @click="approve(p)">
            {{ $t('host.approve') }}
          </UiButton>
        </li>
      </ul>

      <h2 class="group">{{ $t('host.devices') }}</h2>
      <p v-if="clients.length === 0" class="muted">{{ $t('host.noDevices') }}</p>
      <ul class="list">
        <li v-for="c in clients" :key="c.uuid" class="item">
          <div class="who">
            <span class="name">{{ c.name }}</span>
            <span class="meta">{{ c.enabled ? t('host.enabled') : t('host.blocked') }}</span>
          </div>
          <div class="actions">
            <UiButton variant="ghost" :size="32" @click="hostApi.setEnabled(c.uuid, !c.enabled).then(load)">
              {{ c.enabled ? $t('host.block') : $t('host.unblock') }}
            </UiButton>
            <UiButton variant="danger" :size="32" @click="hostApi.unpair(c.uuid).then(load)">
              {{ $t('host.remove') }}
            </UiButton>
          </div>
        </li>
      </ul>
    </template>

    <h2 class="group">{{ $t('host.apps') }}</h2>
    <ul class="apps">
      <li class="app">
        <span class="cover"><Monitor :size="28" :stroke-width="1.75" aria-hidden="true" /></span>
        <span class="name">{{ $t('host.desktop') }}</span>
      </li>
    </ul>
  </section>
</template>

<style lang="scss" scoped>
.host {
  display: grid;
  gap: $sp-4;
  max-width: 760px;

  .title {
    font-size: $fs-h1;
    line-height: 30px;
    font-weight: 650;
  }

  .panel {
    @include surface(2);
    display: grid;
    gap: $sp-2;
    padding: $sp-4 $sp-5;
    border-radius: $r-card;
    transition: box-shadow $t-panel $ease;

    // Host pronto: stato vivo, bordo brace
    &.live {
      box-shadow: inset 0 0 0 1px $ember-500;
    }

    .state {
      display: flex;
      align-items: center;
      gap: $sp-2;
      font-size: $fs-small;
      color: $text-2;

      .dot {
        width: 8px;
        height: 8px;
        border-radius: $r-pill;
        background: $text-3;
      }

      &.starting .dot {
        background: $busy;
      }

      &.ready .dot {
        background: $ok;
      }

      &.error {
        color: $danger;

        .dot {
          background: $danger;
        }
      }
    }

    .end {
      justify-self: start;
    }
  }

  .group {
    @include label;
    margin-top: $sp-4;
  }

  .muted {
    color: $text-3;
    font-size: $fs-small;
  }

  .list {
    display: grid;
    gap: $sp-2;
    margin: 0;
    padding: 0;
    list-style: none;

    .item {
      @include surface(2);
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: $sp-3;
      padding: $sp-3 $sp-4;
      border-radius: $r-card;

      &.request {
        display: grid;
        grid-template-columns: 1fr 1fr 96px auto;
        align-items: end;
      }

      .who {
        display: grid;
        min-width: 0;

        .name {
          @include truncate;
          font-weight: 600;
        }

        .meta {
          font-size: $fs-small;
          color: $text-2;
        }
      }

      .actions {
        display: flex;
        gap: $sp-2;
      }
    }
  }

  .apps {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: $sp-3;
    margin: 0;
    padding: 0;
    list-style: none;

    .app {
      @include surface(2);
      display: grid;
      gap: $sp-2;
      padding: $sp-2;
      border-radius: $r-card;

      .cover {
        display: grid;
        place-items: center;
        aspect-ratio: 3 / 4;
        border-radius: 10px;
        background: $bg-3;
        color: $text-2;
      }

      .name {
        @include truncate;
        padding: 0 $sp-1 $sp-1;
        font-weight: 600;
      }
    }
  }
}
</style>
