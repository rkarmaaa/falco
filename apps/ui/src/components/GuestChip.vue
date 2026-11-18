<script setup lang="ts">
// Ospite collegato nella barra delle azioni. Tasto destro (o clic): dispositivi concessi,
// «Solo guardare», «Espelli». Di default ogni ospite può solo guardare.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Eye, Gamepad2, Keyboard, Mouse, UserX } from 'lucide-vue-next';
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

function toggle(d: Device): void {
  void attempt(t('errors.context.permissions', { name: props.guest.name }), () => setDevice(props.guest.friendId, d, !allowed.value[d]));
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
      :title="t('guests.hint')"
      @click="open"
      @contextmenu.prevent="open"
      @keydown.shift.f10.prevent="openFromKeyboard"
    >
      <UiAvatar :name="guest.name" :src="friend?.avatarUrl" :accent="friend?.accent" :size="32" />
      <span class="info">
        <span class="name">{{ guest.name }}</span>
        <DeviceIcons v-if="!watching" :allowed="allowed" :active="social.activity" :size="13" />
        <span v-else class="watch"><Eye :size="12" :stroke-width="2" aria-hidden="true" />{{ t('guests.watching') }}</span>
      </span>
    </button>

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
        @click="toggle(d.id)"
      >
        <component :is="d.icon" :size="16" :stroke-width="1.9" aria-hidden="true" />
        <span class="grow">{{ t(`devices.${d.id}`) }}</span>
        <Check v-if="allowed[d.id]" class="check" :size="16" :stroke-width="2.4" aria-hidden="true" />
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
    gap: $sp-2;
    height: 48px;
    padding: 0 $sp-3 0 $sp-2;
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
      gap: 1px;
      text-align: left;

      .name {
        @include truncate;
        max-width: 120px;
        font-size: $fs-small;
        font-weight: 600;
        line-height: 16px;
      }

      .watch {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        height: 22px;
        font-size: 12px;
        color: $text-3;
      }
    }
  }
}
</style>
