<script setup lang="ts">
// «Invita»: ruolo con permessi, validità, usi; poi link da copiare
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Copy, Check } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiButton from './UiButton.vue';
import UiSegmented from './UiSegmented.vue';
import PermissionList from './PermissionList.vue';
import { createInvite } from '@/stores/social';
import { isRpcError, type Permissions, type Role } from '@/ipc';

const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

const role = ref<Role>('giocatore');
const validity = ref<'1' | '24' | '168'>('24');
const uses = ref(1);
const link = ref<string | null>(null);
const shared = ref(true);
const copied = ref(false);
const error = ref<string | null>(null);
const busy = ref(false);

// Preset del §7, per l'anteprima a parole
const presets: Record<Role, Permissions> = {
  spettatore: { role: 'spettatore', view: true, audio: true, gamepads: 0, keyboard: false, keyboardSystemKeys: false, mouse: false, launchApps: false, autoJoin: false, clipboard: false, endHostSession: false },
  giocatore: { role: 'giocatore', view: true, audio: true, gamepads: 1, keyboard: false, keyboardSystemKeys: false, mouse: false, launchApps: false, autoJoin: true, clipboard: false, endHostSession: false },
  copilota: { role: 'copilota', view: true, audio: true, gamepads: 2, keyboard: true, keyboardSystemKeys: false, mouse: true, launchApps: false, autoJoin: true, clipboard: false, endHostSession: false },
  fidato: { role: 'fidato', view: true, audio: true, gamepads: 4, keyboard: true, keyboardSystemKeys: true, mouse: true, launchApps: true, autoJoin: true, clipboard: true, endHostSession: true },
};
const roles = computed(() => (['spettatore', 'giocatore', 'copilota', 'fidato'] as Role[]).map((r) => ({ value: r, label: t(`role.${r}`) })));

async function create(): Promise<void> {
  busy.value = true;
  error.value = null;
  try {
    const r = await createInvite(role.value, Number(validity.value), uses.value);
    link.value = r.link;
    shared.value = r.tailscaleShared;
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  } finally {
    busy.value = false;
  }
}

async function copy(): Promise<void> {
  if (!link.value) return;
  try {
    const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
    await writeText(link.value);
  } catch {
    await navigator.clipboard.writeText(link.value);
  }
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}
</script>

<template>
  <UiModal :title="$t('invite.title')" @close="emit('close')">
    <template v-if="!link">
      <div class="field">
        <span class="label">{{ $t('invite.role') }}</span>
        <UiSegmented v-model="role" :label="$t('invite.role')" :options="roles" />
      </div>
      <PermissionList :permissions="presets[role]" />
      <div class="row">
        <div class="field">
          <span class="label">{{ $t('invite.validity') }}</span>
          <UiSegmented
            v-model="validity"
            :label="$t('invite.validity')"
            :options="[
              { value: '1', label: $t('invite.hours1') },
              { value: '24', label: $t('invite.hours24') },
              { value: '168', label: $t('invite.days7') },
            ]"
          />
        </div>
        <label class="field">
          <span class="label">{{ $t('invite.uses') }}</span>
          <input v-model.number="uses" class="number" type="number" min="1" max="10" />
        </label>
      </div>
      <p v-if="uses > 1" class="note">{{ $t('invite.approvalNote') }}</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
    </template>

    <template v-else>
      <p class="note">{{ $t('invite.share') }}</p>
      <div class="link">
        <code class="code">{{ link }}</code>
        <UiButton variant="primary" @click="copy">
          <Check v-if="copied" :size="18" :stroke-width="1.75" aria-hidden="true" />
          <Copy v-else :size="18" :stroke-width="1.75" aria-hidden="true" />
          {{ copied ? $t('invite.copied') : $t('invite.copy') }}
        </UiButton>
      </div>
      <p class="warn">{{ $t('invite.secret') }}</p>
      <p v-if="!shared" class="warn">{{ $t('invite.noShare') }}</p>
    </template>

    <template #actions>
      <UiButton v-if="!link" variant="primary" :disabled="busy" @click="create">{{ $t('invite.create') }}</UiButton>
      <UiButton v-else variant="secondary" @click="emit('close')">{{ $t('invite.done') }}</UiButton>
    </template>
  </UiModal>
</template>

<style lang="scss" scoped>
.field {
  display: grid;
  gap: $sp-1;

  .label {
    @include label;
  }
}

.row {
  display: flex;
  flex-wrap: wrap;
  gap: $sp-4;
}

.number {
  @include surface(2);
  @include focus-ring;
  width: 80px;
  height: $control-h;
  padding: 0 $sp-3;
  border-radius: $r-field;
  color: $text-1;
  font: inherit;
}

.note {
  color: $text-2;
  font-size: $fs-small;
}

.warn {
  color: $busy;
  font-size: $fs-small;
}

.error {
  color: $danger;
  font-size: $fs-small;
}

.link {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: $sp-3;

  .code {
    @include surface(2);
    @include truncate;
    padding: $sp-2 $sp-3;
    border-radius: $r-field;
    font-size: $fs-small;
    user-select: all;
  }
}
</style>
