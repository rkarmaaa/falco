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
    <UiSwitch :model-value="eff.gamepads > 0" :label="$t('perms.gamepads')" @update:model-value="setOverride('gamepads', $event ? 1 : 0)" />
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


  .note {
    @include message($text-3);
  }
}
</style>
