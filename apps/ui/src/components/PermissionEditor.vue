<script setup lang="ts">
// Ruolo e permessi granulari di un amico (§7). L'host li cambia anche durante la sessione.
import { onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import UiSegmented from './UiSegmented.vue';
import UiSwitch from './UiSwitch.vue';
import { rpc, type Permissions, type Role } from '@/ipc';

const props = defineProps<{ friendId: string }>();
const { t } = useI18n();

interface Loaded {
  role: Role | null;
  overrides: Record<string, unknown> | null;
  effective: Permissions | null;
  paired: boolean;
}

const role = ref<Role>('spettatore');
const eff = ref<Permissions | null>(null);
const paired = ref(false);
const roles = (['spettatore', 'giocatore', 'copilota', 'fidato'] as Role[]).map((r) => ({ value: r, label: t(`role.${r}`) }));

async function load(): Promise<void> {
  const r = await rpc<Loaded>('friend.getPermissions', { id: props.friendId });
  role.value = r.role ?? 'spettatore';
  eff.value = r.effective;
  paired.value = r.paired;
}

async function setRole(r: Role): Promise<void> {
  // Nuovo ruolo: si azzerano le modifiche granulari
  await rpc('friend.setPermissions', { id: props.friendId, role: r, overrides: {} });
  await load();
}

async function setOverride(key: keyof Permissions, value: boolean | number): Promise<void> {
  const r = await rpc<Loaded>('friend.getPermissions', { id: props.friendId });
  const overrides = { ...(r.overrides ?? {}), [key]: value };
  await rpc('friend.setPermissions', { id: props.friendId, overrides });
  await load();
}

watch(role, (r, old) => {
  if (old && r !== eff.value?.role) void setRole(r);
});
watch(() => props.friendId, load);
onMounted(load);
</script>

<template>
  <section v-if="eff" class="perms">
    <h3 class="label">{{ $t('perms.title') }}</h3>
    <UiSegmented v-model="role" :label="$t('invite.role')" :options="roles" />
    <UiSwitch :model-value="eff.autoJoin" :label="$t('perms.autoJoin')" @update:model-value="setOverride('autoJoin', $event)" />
    <UiSwitch :model-value="eff.keyboard" :label="$t('perms.keyboard')" @update:model-value="setOverride('keyboard', $event)" />
    <UiSwitch :model-value="eff.mouse" :label="$t('perms.mouse')" @update:model-value="setOverride('mouse', $event)" />
    <UiSwitch :model-value="eff.launchApps" :label="$t('perms.launchApps')" @update:model-value="setOverride('launchApps', $event)" />
    <label class="pads">
      <span>{{ $t('perms.gamepads') }}</span>
      <input type="range" min="0" max="4" :value="eff.gamepads" @change="setOverride('gamepads', Number(($event.target as HTMLInputElement).value))" />
      <span class="n">{{ eff.gamepads }}</span>
    </label>
    <p class="note">{{ paired ? $t('perms.enforced') : $t('perms.notPaired') }}</p>
  </section>
</template>

<style lang="scss" scoped>
.perms {
  align-self: stretch;
  display: grid;
  gap: $sp-2;
  margin-top: $sp-5;

  .label {
    @include label;
  }

  .pads {
    display: grid;
    grid-template-columns: 1fr 96px 16px;
    align-items: center;
    gap: $sp-2;
    font-weight: 600;

    input {
      accent-color: $accent-500;
    }

    .n {
      font-variant-numeric: tabular-nums;
      color: $text-2;
    }
  }

  .note {
    font-size: $fs-small;
    color: $text-3;
  }
}
</style>
