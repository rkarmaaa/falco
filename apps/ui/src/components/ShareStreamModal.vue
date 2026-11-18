<script setup lang="ts">
// «Condividi stream»: cosa può fare chi entra, per quanto e quante persone; poi un link corto
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Copy, Check } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiButton from './UiButton.vue';
import UiSegmented from './UiSegmented.vue';
import PermissionList from './PermissionList.vue';
import { shareStream } from '@/stores/social';
import { core, setHostAvailable } from '@/stores/core';
import { attempt } from '@/stores/errors';
import { copyText } from '@/utils/clipboard';
import { isRpcError, type Permissions, type Role } from '@/ipc';

const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

const role = ref<Role>('spettatore');
const validity = ref<'1' | '24' | '168'>('24');
const uses = ref(1);
const link = ref<string | null>(null);
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
    link.value = (await shareStream(role.value, Number(validity.value), uses.value)).url;
    // Un solo clic: il link è pronto e la condivisione parte da sola
    if (core.host.state === 'stopped' || core.host.state === 'error') {
      void attempt(t('errors.context.share'), () => setHostAvailable(true));
    }
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  } finally {
    busy.value = false;
  }
}

onMounted(() => void create());

async function copy(): Promise<void> {
  if (!link.value || !(await copyText(link.value))) return;
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}
</script>

<template>
  <UiModal :title="$t('stream.title')" @close="emit('close')">
    <template v-if="!link">
      <div class="field">
        <span class="label">{{ $t('stream.role') }}</span>
        <UiSegmented v-model="role" :label="$t('stream.role')" :options="roles" />
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
          <span class="label">{{ $t('stream.uses') }}</span>
          <input v-model.number="uses" class="number" type="number" min="1" max="10" />
        </label>
      </div>
      <p v-if="uses > 1" class="note">{{ $t('invite.approvalNote') }}</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
    </template>

    <template v-else>
      <p class="note">{{ $t('stream.shareHint') }}</p>
      <div class="link">
        <code class="code">{{ link }}</code>
        <UiButton variant="primary" @click="copy">
          <Check v-if="copied" :size="18" :stroke-width="1.75" aria-hidden="true" />
          <Copy v-else :size="18" :stroke-width="1.75" aria-hidden="true" />
          {{ copied ? $t('invite.copied') : $t('invite.copy') }}
        </UiButton>
      </div>
      <p class="warn">{{ $t('stream.secret') }}</p>
    </template>

    <template #actions>
      <UiButton v-if="!link" variant="primary" :disabled="busy" @click="create">{{ busy ? $t('stream.creating') : $t('stream.create') }}</UiButton>
      <template v-else>
        <UiButton variant="ghost" @click="link = null">{{ $t('stream.options') }}</UiButton>
        <UiButton variant="secondary" @click="emit('close')">{{ $t('invite.done') }}</UiButton>
      </template>
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
  @include message($text-2);
}

.warn {
  @include message($busy);
}

.error {
  @include message($danger);
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
    line-height: $lh-small;
    user-select: all;
  }
}
</style>
