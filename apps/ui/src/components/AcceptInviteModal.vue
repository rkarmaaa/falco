<script setup lang="ts">
// «Hai un invito?»: un campo per link e codici. Riconosce codice o link amico, link di stream
// e inviti firmati; per questi ultimi mostra host e permessi, poi i passi con spunta.
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { CircleCheck, CircleDashed, CircleX, ClipboardPaste, LoaderCircle, Radio, Ticket, UserPlus } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiButton from './UiButton.vue';
import UiAvatar from './UiAvatar.vue';
import PermissionList from './PermissionList.vue';
import { acceptInvite, addFriend, findInviteLink, previewInvite, resetAccept, social, type InviteStep } from '@/stores/social';
import { isRpcError, type InvitePreview } from '@/ipc';
import { joinStream, ui } from '@/stores/ui';
import { joinFriend } from '@/stores/core';
import { attempt } from '@/stores/errors';

const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

const text = ref(ui.accept.link ?? '');
const preview = ref<InvitePreview | null>(null);
// L'host di un invito è spesso già un amico: si usa la sua foto
const hostAvatar = computed(() => social.friends.find((f) => f.id === preview.value?.host.id)?.avatarUrl);
const error = ref<string | null>(null);
const link = computed(() => findInviteLink(text.value));
const sent = ref<string | null>(null);
const busy = ref(false);

/** Cosa ha incollato l'utente. */
const kind = computed<'invite' | 'stream' | 'friend' | null>(() => {
  const v = text.value.trim();
  if (link.value) return 'invite';
  if (/falco:\/\/join\/|\/j\/[A-Za-z2-7]{8}/.test(v)) return 'stream';
  if (/falco:\/\/add\/|\/a\/[A-Za-z2-7]{8}/.test(v) || /^#?[A-Za-z2-7]{4}[\s-]?[A-Za-z2-7]{4}$/.test(v)) return 'friend';
  return null;
});
const code = computed(() => {
  const m = /^#?([A-Za-z2-7]{4})[\s-]?([A-Za-z2-7]{4})$/.exec(text.value.trim());
  return m ? `${m[1]}-${m[2]}`.toUpperCase() : '';
});

async function paste(): Promise<void> {
  try {
    const { readText } = await import('@tauri-apps/plugin-clipboard-manager');
    text.value = (await readText()) ?? '';
  } catch {
    text.value = await navigator.clipboard.readText().catch(() => text.value);
  }
  sent.value = null;
  await check();
}

async function primary(): Promise<void> {
  error.value = null;
  if (kind.value === 'invite') return accept();
  if (kind.value === 'stream') return joinStream(text.value);
  if (kind.value !== 'friend') return;
  busy.value = true;
  try {
    const r = await addFriend(text.value);
    sent.value = t(r.theyAsked ? 'addFriend.theyAsked' : 'addFriend.sent', { name: r.user.name });
  } catch (e) {
    error.value = isRpcError(e) ? e.message : t('connect.genericError');
  } finally {
    busy.value = false;
  }
}
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

// Link di stream: appena si è amici si entra nella sessione dell'host
watch(
  () => social.accept?.done,
  (done) => {
    const host = ui.accept.joinHost;
    if (!done || !host) return;
    close();
    void attempt(t('errors.context.join', { name: host.name }), () => joinFriend(host.id, host.name), { name: host.name });
  },
);

onMounted(async () => {
  await check();
  if (ui.accept.auto && preview.value) await accept();
});
</script>

<template>
  <UiModal :title="social.accept?.done ? $t('accept.friends') : $t('accept.title')" @close="close">
    <template v-if="!social.accept">
      <p class="intro">{{ $t('accept.intro') }}</p>
      <div class="paste">
        <input
          v-model="text"
          class="input"
          spellcheck="false"
          autocomplete="off"
          :aria-label="$t('accept.paste')"
          :placeholder="$t('accept.placeholder')"
          @input="(sent = null), check()"
          @keydown.enter="kind && primary()"
        />
        <UiButton variant="secondary" :size="40" @click="paste">
          <ClipboardPaste :size="16" :stroke-width="1.75" aria-hidden="true" />
          {{ $t('accept.pasteButton') }}
        </UiButton>
      </div>

      <div v-if="kind === 'friend'" class="kind">
        <span class="icon"><UserPlus :size="20" :stroke-width="1.75" aria-hidden="true" /></span>
        <div class="who">
          <span class="name">{{ $t('accept.kind.friend') }}</span>
          <span class="tag">{{ code ? `#${code}` : $t('accept.kind.friendLink') }}</span>
        </div>
      </div>
      <div v-else-if="kind === 'stream'" class="kind">
        <span class="icon live"><Radio :size="20" :stroke-width="1.75" aria-hidden="true" /></span>
        <div class="who">
          <span class="name">{{ $t('accept.kind.stream') }}</span>
          <span class="tag">{{ $t('accept.kind.streamBody') }}</span>
        </div>
      </div>
      <div v-else-if="!text.trim()" class="kinds">
        <span><UserPlus :size="16" :stroke-width="1.75" aria-hidden="true" /> {{ $t('accept.kind.friend') }}</span>
        <span><Radio :size="16" :stroke-width="1.75" aria-hidden="true" /> {{ $t('accept.kind.stream') }}</span>
        <span><Ticket :size="16" :stroke-width="1.75" aria-hidden="true" /> {{ $t('accept.kind.invite') }}</span>
      </div>
      <div v-if="preview" class="host">
        <UiAvatar :name="preview.host.name" :src="hostAvatar" :accent="preview.host.accent" :size="56" />
        <div class="who">
          <span class="name">{{ preview.host.name }}</span>
          <span class="tag">{{ preview.host.tag }} · {{ $t(`role.${preview.role}`) }}</span>
        </div>
      </div>
      <PermissionList v-if="preview" :permissions="preview.permissions" />
      <p v-if="preview?.needsTailscaleShare" class="note">{{ $t('accept.browserNote') }}</p>
      <p v-if="sent" class="done">{{ sent }}</p>
      <p v-if="error" class="error" role="alert">{{ error }}</p>
      <p v-else-if="text.trim() && !kind" class="note">{{ $t('accept.unknown') }}</p>
    </template>

    <template v-else>
      <ol class="steps">
        <li v-for="s in steps" :key="s" class="step" :class="{ ok: social.accept.steps[s] === true, fail: social.accept.steps[s] === false }">
          <CircleCheck v-if="social.accept.steps[s] === true" :size="20" :stroke-width="1.75" aria-hidden="true" />
          <CircleX v-else-if="social.accept.steps[s] === false" :size="20" :stroke-width="1.75" aria-hidden="true" />
          <LoaderCircle
            v-else-if="!social.accept.error && steps.indexOf(s) === steps.findIndex((x) => social.accept!.steps[x] === null)"
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
      <UiButton v-if="!social.accept" variant="primary" :disabled="!kind || (kind === 'invite' && !preview) || busy || !!sent" @click="primary">
        {{ kind === 'friend' ? $t('addFriend.send') : kind === 'stream' ? $t('stream.watch') : $t('accept.accept') }}
      </UiButton>
      <UiButton v-else variant="secondary" @click="close">{{ social.accept.done ? $t('invite.done') : $t('app.close') }}</UiButton>
    </template>
  </UiModal>
</template>

<style lang="scss" scoped>
.intro {
  @include message;
}

.paste {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: $sp-2;

  .input {
    @include surface(2);
    @include focus-ring;
    height: $control-h;
    min-width: 0;
    padding: 0 $sp-3;
    border-radius: $r-field;
    color: $text-1;
    font: inherit;
  }
}

.kinds {
  display: flex;
  flex-wrap: wrap;
  gap: $sp-2 $sp-4;
  color: $text-3;
  font-size: $fs-small;

  span {
    display: inline-flex;
    align-items: center;
    gap: $sp-1;
  }
}

.kind {
  @include surface(2);
  display: flex;
  align-items: center;
  gap: $sp-3;
  padding: $sp-3 $sp-4;
  border-radius: $r-card;

  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: $r-pill;
    background: $accent-soft;
    color: $accent-fg;

    &.live {
      background: $live-soft;
      color: $live;
    }
  }

  .who {
    display: grid;

    .name {
      font-weight: $fw-strong;
    }

    .tag {
      @include message;
    }
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
  color: $accent-fg;
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
