<script setup lang="ts">
// Permessi descritti a parole, con icona: lo stato non è mai solo colore
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Minus } from 'lucide-vue-next';
import type { Permissions } from '@/ipc';

const props = defineProps<{ permissions: Permissions }>();
const { t } = useI18n();

const items = computed(() => {
  const p = props.permissions;
  return [
    { ok: p.view, text: t('perm.view') },
    { ok: p.gamepads > 0, text: p.gamepads > 0 ? t('perm.gamepads', { n: p.gamepads }, p.gamepads) : t('perm.noGamepads') },
    { ok: p.keyboard, text: p.keyboard ? (p.keyboardSystemKeys ? t('perm.keyboardFull') : t('perm.keyboard')) : t('perm.noKeyboard') },
    { ok: p.mouse, text: p.mouse ? t('perm.mouse') : t('perm.noMouse') },
    { ok: p.autoJoin, text: p.autoJoin ? t('perm.autoJoin') : t('perm.knock') },
    { ok: p.launchApps, text: p.launchApps ? t('perm.launchApps') : t('perm.noLaunchApps') },
  ];
});
</script>

<template>
  <ul class="perms">
    <li v-for="(it, i) in items" :key="i" class="perm" :class="{ ok: it.ok }">
      <Check v-if="it.ok" :size="16" :stroke-width="1.75" aria-hidden="true" />
      <Minus v-else :size="16" :stroke-width="1.75" aria-hidden="true" />
      <span>{{ it.text }}</span>
    </li>
  </ul>
</template>

<style lang="scss" scoped>
.perms {
  display: grid;
  gap: $sp-1;
  margin: 0;
  padding: 0;
  list-style: none;

  .perm {
    display: flex;
    align-items: center;
    gap: $sp-2;
    font-size: $fs-small;
    color: $text-3;

    &.ok {
      color: $text-1;

      svg {
        color: $ok;
      }
    }
  }
}
</style>
