<script setup lang="ts">
// Ospite collegato nella barra delle azioni. Tasto destro (o clic): dispositivi concessi,
// «Solo guardare», «Espelli». Di default ogni ospite può solo guardare.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Eye, Gamepad2, Keyboard, LoaderCircle, Mouse, UserX } from 'lucide-vue-next';
import UiAvatar from './UiAvatar.vue';
import DeviceIcons from './DeviceIcons.vue';
import ContextMenu from './ContextMenu.vue';
import { friendApi, setDevice, social, watchOnly, type Device } from '@/stores/social';
import { attempt } from '@/stores/errors';

const props = defineProps<{ guest: { friendId: string; name: string; since: number } }>();
const { t } = useI18n();
const menu = ref<{ x: number; y: number } | null>(null);

const friend = computed(() => social.friends.find((f) => f.id === props.guest.friendId));
const perms = computed(() => social.guestPerms[props.guest.friendId]);
const allowed = computed<Record<Device, boolean>>(() => ({
  keyboard: !!perms.value?.keyboard,
  mouse: !!perms.value?.mouse,
  controller: (perms.value?.gamepads ?? 0) > 0,
}));
// Nuvoletta con nome e latenza, centrata sopra la foto
const tip = ref<{ x: number; y: number } | null>(null);
function showTip(e: Event): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  tip.value = { x: r.left + r.width / 2, y: r.top - 8 };
}

const pingLevel = computed(() => {
  const ms = friend.value?.rttMs;
  return ms == null ? '' : ms < 40 ? 'good' : ms <= 90 ? 'fair' : 'bad';
});
const watching = computed(() => !allowed.value.keyboard && !allowed.value.mouse && !allowed.value.controller);

function open(e: MouseEvent): void {
  menu.value = { x: e.clientX, y: e.clientY };
}

function openFromKeyboard(e: KeyboardEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  menu.value = { x: r.left, y: r.top };
}

const devices = [
  { id: 'keyboard' as const, icon: Keyboard },
  { id: 'mouse' as const, icon: Mouse },
  { id: 'controller' as const, icon: Gamepad2 },
];

// Dispositivo in corso di applicazione: rotella finché l'host non conferma
const pending = ref<Device | null>(null);
async function toggle(d: Device): Promise<void> {
  if (pending.value) return;
  pending.value = d;
  try {
    await attempt(t('errors.context.permissions', { name: props.guest.name }), () => setDevice(props.guest.friendId, d, !allowed.value[d]));
  } finally {
    pending.value = null;
  }
}

function only(): void {
  menu.value = null;
  void attempt(t('errors.context.permissions', { name: props.guest.name }), () => watchOnly(props.guest.friendId));
}

function kick(): void {
  menu.value = null;
  void attempt(t('errors.context.kick', { name: props.guest.name }), () => friendApi.kick(props.guest.friendId));
}
</script>

<template>
  <li class="chip" :class="{ active: Object.values(social.activity).some(Boolean) && !watching }">
    <button
      type="button"
      class="main"
      :aria-haspopup="'menu'"
      :aria-expanded="!!menu"
      :aria-label="`${guest.name} · ${t('guests.hint')}`"
      @click="open"
      @contextmenu.prevent="open"
      @keydown.shift.f10.prevent="openFromKeyboard"
      @mouseenter="showTip"
      @mouseleave="tip = null"
      @focus="showTip"
      @blur="tip = null"
    >

      <UiAvatar :name="guest.name" :src="friend?.avatarUrl" :accent="friend?.accent" :size="40" />
      <span class="info">
        <DeviceIcons vertical :allowed="allowed" :active="social.activity" :size="11" />
      </span>
    </button>

    <Teleport to="body">
      <div v-if="tip && !menu" class="guest-tip" role="tooltip" :style="{ left: `${tip.x}px`, top: `${tip.y}px` }">
        <strong>{{ guest.name }}</strong>
        <span :class="pingLevel">{{ friend?.rttMs != null ? `${friend.rttMs} ms` : '–' }}</span>
      </div>
    </Teleport>
    <ContextMenu v-if="menu" :x="menu.x" :y="menu.y" :label="t('guests.menu', { name: guest.name })" @close="menu = null">
      <div class="head">
        <strong>{{ guest.name }}</strong>
        <span>{{ t('guests.since', { min: Math.max(1, Math.round((Date.now() / 1000 - guest.since) / 60)) }) }}</span>
      </div>
      <hr />
      <button
        v-for="d in devices"
        :key="d.id"
        type="button"
        class="item"
        role="menuitemcheckbox"
        :aria-checked="allowed[d.id]"
        :aria-busy="pending === d.id"
        :disabled="pending !== null"
        @click="toggle(d.id)"
      >
        <component :is="d.icon" :size="16" :stroke-width="1.9" aria-hidden="true" />
        <span class="grow">{{ t(`devices.${d.id}`) }}</span>
        <LoaderCircle v-if="pending === d.id" class="check spin" :size="16" :stroke-width="2.4" aria-hidden="true" />
        <Check v-else-if="allowed[d.id]" class="check" :size="16" :stroke-width="2.4" aria-hidden="true" />
      </button>
      <hr />
      <button type="button" class="item" role="menuitem" :disabled="watching" @click="only">
        <Eye :size="16" :stroke-width="1.9" aria-hidden="true" />
        <span class="grow">{{ t('guests.watchOnly') }}</span>
      </button>
      <button type="button" class="item danger" role="menuitem" @click="kick">
        <UserX :size="16" :stroke-width="1.9" aria-hidden="true" />
        <span class="grow">{{ t('guests.kick') }}</span>
      </button>
    </ContextMenu>
  </li>
</template>

<style lang="scss" scoped>
.chip {
  list-style: none;

  .main {
    @include reset-button;
    @include focus-ring;
    display: flex;
    align-items: center;
    // Foto e colonna delle icone con lo stesso margine: 4 px sopra e sotto, 6 px tra le due
    gap: 6px;
    height: 48px;
    padding: 0 6px 0 4px;
    border: 1px solid $line;
    border-radius: $r-card;
    background: $bg-2;
    transition:
      background $t-fast $ease,
      border-color $t-fast $ease;

    &:hover,
    &[aria-expanded='true'] {
      border-color: $line-strong;
      background: $bg-3;
    }

    .info {
      display: grid;
      align-content: center;
      text-align: left;

      .name {
        @include truncate;
        max-width: 120px;
        font-size: $fs-small;
        font-weight: 600;
        line-height: 16px;
      }
    }
  }
}

.spin {
  animation: guest-spin 0.9s linear infinite;

  @media (prefers-reduced-motion: reduce) {
    animation: none;
  }
}

@keyframes guest-spin {
  to {
    transform: rotate(360deg);
  }
}

// Nome e latenza sopra la foto (in un livello sopra la pagina, la barra non la taglia)
.guest-tip {
  position: fixed;
  z-index: 70;
  display: grid;
  justify-items: center;
  gap: 2px;
  padding: $sp-1 $sp-3;
  border: 1px solid $line-strong;
  border-radius: $r-button;
  background: $bg-float;
  backdrop-filter: blur(12px);
  box-shadow: $shadow-2;
  font-size: $fs-small;
  white-space: nowrap;
  pointer-events: none;
  transform: translate(-50%, -100%);

  .good {
    color: $ok;
  }

  .fair {
    color: $busy;
  }

  .bad {
    color: $danger;
  }
}
</style>
