<script setup lang="ts">
// Ospita: condividi lo schermo, scegli la qualità, gestisci chi è collegato e i dispositivi associati.
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Gamepad2, Keyboard, Monitor, Mouse } from 'lucide-vue-next';
import UiButton from '@/components/UiButton.vue';
import UiField from '@/components/UiField.vue';
import UiAvatar from '@/components/UiAvatar.vue';
import ShareSettings from '@/components/ShareSettings.vue';
import { core, hostApi } from '@/stores/core';
import { friendApi, loadGuests, setDevice, social, type Device } from '@/stores/social';
import { attempt } from '@/stores/errors';
import type { PairedClient, PendingPairing } from '@/ipc';
import UiPage from '@/components/UiPage.vue';

const { t } = useI18n();
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
      void attempt(t('errors.context.hostLists'), () => Promise.all([load(), loadGuests()]));
      timer = setInterval(() => void load().catch(() => {}), 3000);
    } else {
      pairings.value = [];
      clients.value = [];
    }
  },
  { immediate: true },
);
onBeforeUnmount(() => clearInterval(timer));

async function approve(p: PendingPairing): Promise<void> {
  await attempt(t('errors.context.pairing'), async () => {
    await hostApi.approve(p.id, pins.value[p.id] ?? '', names.value[p.id] || p.name);
    await load();
  });
}

const devices = [
  { id: 'keyboard' as const, icon: Keyboard },
  { id: 'mouse' as const, icon: Mouse },
  { id: 'controller' as const, icon: Gamepad2 },
];

function allowed(id: string): Record<Device, boolean> {
  const p = social.guestPerms[id];
  return { keyboard: !!p?.keyboard, mouse: !!p?.mouse, controller: (p?.gamepads ?? 0) > 0 };
}

function toggle(id: string, name: string, d: Device): void {
  void attempt(t('errors.context.permissions', { name }), () => setDevice(id, d, !allowed(id)[d]));
}
</script>

<template>
  <UiPage class="host" :title="t('nav.host')">
    <p v-if="core.host.state === 'error'" class="host-error" role="alert">
      {{ t('host.state.error') }}<template v-if="core.host.message"> · {{ core.host.message }}</template>
    </p>

    <ShareSettings />

    <template v-if="ready">
      <section class="block">
        <h2>{{ t('host.guests') }}</h2>
        <p v-if="social.guests.length === 0" class="muted">{{ t('host.noGuests') }}</p>
        <ul class="list">
          <li v-for="g in social.guests" :key="g.friendId" class="item">
            <UiAvatar :name="g.name" :src="social.friends.find((f) => f.id === g.friendId)?.avatarUrl" :accent="social.friends.find((f) => f.id === g.friendId)?.accent" :size="32" />
            <div class="who">
              <span class="name">{{ g.name }}</span>
              <span class="meta">{{ t('host.since', { min: Math.max(1, Math.round((Date.now() / 1000 - g.since) / 60)) }) }}</span>
            </div>
            <div class="devices" role="group" :aria-label="t('guests.devicesOf', { name: g.name })">
              <button
                v-for="d in devices"
                :key="d.id"
                type="button"
                class="dev"
                :class="{ live: allowed(g.friendId)[d.id] && social.activity[d.id] }"
                :aria-pressed="allowed(g.friendId)[d.id]"
                @click="toggle(g.friendId, g.name, d.id)"
              >
                <component :is="d.icon" :size="14" :stroke-width="1.9" aria-hidden="true" />
                {{ t(`devices.${d.id}`) }}
              </button>
            </div>
            <UiButton variant="danger" :size="32" @click="attempt(t('errors.context.kick', { name: g.name }), () => friendApi.kick(g.friendId))">{{ t('guests.kick') }}</UiButton>
          </li>
        </ul>
      </section>

      <section v-if="pairings.length" class="block">
        <h2>{{ t('host.requests') }}</h2>
        <ul class="list">
          <li v-for="p in pairings" :key="p.id" class="item request">
            <div class="who">
              <span class="name">{{ p.name }}</span>
              <span class="meta">{{ p.address }}</span>
            </div>
            <UiField v-model="names[p.id]!" :label="t('host.deviceName')" :placeholder="p.name" />
            <UiField v-model="pins[p.id]!" :label="t('host.pin')" inputmode="numeric" :maxlength="4" placeholder="0000" />
            <UiButton variant="primary" :disabled="(pins[p.id] ?? '').length !== 4" @click="approve(p)">{{ t('host.approve') }}</UiButton>
          </li>
        </ul>
      </section>

      <section class="block">
        <h2>{{ t('host.devices') }}</h2>
        <p v-if="clients.length === 0" class="muted">{{ t('host.noDevices') }}</p>
        <ul class="list">
          <li v-for="c in clients" :key="c.uuid" class="item">
            <Monitor :size="18" :stroke-width="1.8" class="dim" aria-hidden="true" />
            <div class="who">
              <span class="name">{{ c.name }}</span>
              <span class="meta">{{ c.enabled ? t('host.enabled') : t('host.blocked') }}</span>
            </div>
            <div class="row">
              <UiButton variant="danger" :size="32" @click="attempt(t('errors.context.device'), () => hostApi.unpair(c.uuid).then(load))">{{ t('host.remove') }}</UiButton>
            </div>
          </li>
        </ul>
      </section>
    </template>
  </UiPage>
</template>

<style lang="scss" scoped>
.host {
  display: grid;
  gap: $sp-7;


  .block {
    display: grid;
    gap: $sp-3;

    h2 {
      @include label;
    }
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
      display: flex;
      align-items: center;
      gap: $sp-3;
      padding: $sp-3 $sp-4;
      border: 1px solid $line;
      border-radius: $r-card;
      background: $bg-2;

      &.request {
        display: grid;
        grid-template-columns: 1fr 1fr 96px auto;
        align-items: end;
      }

      .dim {
        color: $text-3;
      }

      .who {
        display: grid;
        flex: 1;
        min-width: 0;

        .name {
          @include truncate;
          font-weight: 600;
        }

        .meta {
          font-size: $fs-small;
          color: $text-3;
        }
      }

      .row {
        display: flex;
        gap: $sp-2;
      }

      .devices {
        display: flex;
        gap: $sp-1;

        .dev {
          @include reset-button;
          @include focus-ring;
          display: flex;
          align-items: center;
          gap: 4px;
          height: 32px;
          padding: 0 $sp-3 0 $sp-1;
          border: 1px solid $line;
          border-radius: $r-pill;
          color: $text-3;
          font-size: 12px;

          padding: 0 $sp-3 0 10px;
          transition:
            background $t-fast $ease,
            color $t-fast $ease;

          &[aria-pressed='true'] {
            border-color: rgba(157, 169, 255, 0.35);
            background: $accent-soft;
            color: $text-1;
          }

          // Input in arrivo
          &.live {
            border-color: rgba(70, 224, 169, 0.5);
            background: $live-soft;
            color: $live;
            box-shadow: 0 0 14px $live-glow;
          }
        }
      }
    }
  }
}

.host-error {
  @include message($danger);
  padding: $sp-3 $sp-4;
  border: 1px solid $danger-line;
  border-radius: $r-card;
  background: $danger-soft;
}
</style>
