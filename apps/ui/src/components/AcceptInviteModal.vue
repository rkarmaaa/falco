<script setup lang="ts">
// «Accetta invito»: scheda dell'host, permessi a parole, poi tre passi con spunta
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { CircleCheck, CircleDashed, CircleX, LoaderCircle } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiButton from './UiButton.vue';
import UiAvatar from './UiAvatar.vue';
import PermissionList from './PermissionList.vue';
import { acceptInvite, findInviteLink, previewInvite, resetAccept, social, type InviteStep } from '@/stores/social';
import { isRpcError, type InvitePreview } from '@/ipc';

const props = defineProps<{ initialLink?: string | null }>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

const text = ref(props.initialLink ?? '');
const preview = ref<InvitePreview | null>(null);
const error = ref<string | null>(null);
const link = computed(() => findInviteLink(text.value));
const steps: InviteStep[] = ['tailscale', 'share', 'handshake', 'pairing'];

async function check(): Promise<void> {
  error.value = null;
  preview.value = null;
  if (!link.value) return;
  try {
    preview.value = await previewInvite(link.value);
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('accept.invalid');
  }
}

async function accept(): Promise<void> {
  if (!link.value) return;
  try {
    await acceptInvite(link.value);
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  }
}

function close(): void {
  resetAccept();
  emit('close');
}

onMounted(() => void check());
</script>

<template>
  <UiModal :title="social.accept?.done ? $t('accept.friends') : $t('accept.title')" @close="close">
    <template v-if="!social.accept">
      <label class="paste">
        <span class="label">{{ $t('accept.paste') }}</span>
        <textarea v-model="text" rows="3" spellcheck="false" placeholder="falco://invite/…" @input="check" />
      </label>
      <div v-if="preview" class="host">
        <UiAvatar :name="preview.host.name" :accent="preview.host.accent" :size="56" />
        <div class="who">
          <span class="name">{{ preview.host.name }}</span>
          <span class="tag">{{ preview.host.tag }} · {{ $t(`role.${preview.role}`) }}</span>
        </div>
      </div>
      <PermissionList v-if="preview" :permissions="preview.permissions" />
      <p v-if="preview?.needsTailscaleShare" class="note">{{ $t('accept.browserNote') }}</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
    </template>

    <template v-else>
      <ol class="steps">
        <li v-for="s in steps" :key="s" class="step" :class="{ ok: social.accept.steps[s] === true, fail: social.accept.steps[s] === false }">
          <CircleCheck v-if="social.accept.steps[s] === true" :size="20" :stroke-width="1.75" aria-hidden="true" />
          <CircleX v-else-if="social.accept.steps[s] === false" :size="20" :stroke-width="1.75" aria-hidden="true" />
          <LoaderCircle
            v-else-if="steps.indexOf(s) === steps.findIndex((x) => social.accept!.steps[x] === null)"
            class="spin"
            :size="20"
            :stroke-width="1.75"
            aria-hidden="true"
          />
          <CircleDashed v-else :size="20" :stroke-width="1.75" aria-hidden="true" />
          <span>{{ $t(`accept.step.${s}`) }}</span>
        </li>
      </ol>
      <p v-if="social.accept.error" class="error" role="alert">{{ social.accept.error }}</p>
      <p v-if="social.accept.done" class="done">{{ $t('accept.doneBody', { name: preview?.host.name ?? '' }) }}</p>
    </template>

    <template #actions>
      <UiButton v-if="!social.accept" variant="primary" :disabled="!preview" @click="accept">{{ $t('accept.accept') }}</UiButton>
      <UiButton v-else variant="secondary" @click="close">{{ social.accept.done ? $t('invite.done') : $t('app.close') }}</UiButton>
    </template>
  </UiModal>
</template>

<style lang="scss" scoped>
.paste {
  display: grid;
  gap: $sp-1;

  .label {
    @include label;
  }

  textarea {
    @include surface(2);
    @include focus-ring;
    padding: $sp-2 $sp-3;
    border-radius: $r-field;
    color: $text-1;
    font: #{$fs-small}/18px $font;
    resize: none;
  }
}

.host {
  display: flex;
  align-items: center;
  gap: $sp-3;

  .who {
    display: grid;

    .name {
      font-size: $fs-h2;
      font-weight: 600;
    }

    .tag {
      color: $text-2;
      font-size: $fs-small;
    }
  }
}

.steps {
  display: grid;
  gap: $sp-3;
  margin: 0;
  padding: 0;
  list-style: none;

  .step {
    display: flex;
    align-items: center;
    gap: $sp-3;
    color: $text-2;

    &.ok {
      color: $text-1;

      svg {
        color: $ok;
      }
    }

    &.fail svg {
      color: $danger;
    }
  }
}

.spin {
  color: $accent-500;
  animation: spin 1s linear infinite;

  @media (prefers-reduced-motion: reduce) {
    animation: none;
  }
}

.note {
  color: $text-2;
  font-size: $fs-small;
}

.error {
  color: $danger;
  font-size: $fs-small;
}

.done {
  color: $ok;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
