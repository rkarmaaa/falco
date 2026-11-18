<script setup lang="ts">
// «Aggiungi amico»: il mio codice e il mio link da condividere, poi codice o link dell'amico
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Copy, Link2, Send } from 'lucide-vue-next';
import UiModal from './UiModal.vue';
import UiButton from './UiButton.vue';
import UiField from './UiField.vue';
import FriendRequests from './FriendRequests.vue';
import { addFriend, social } from '@/stores/social';
import { ui } from '@/stores/ui';
import { isRpcError } from '@/ipc';
import { copyText } from '@/utils/clipboard';

const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();

const text = ref(ui.addFriendText);
const busy = ref(false);
const result = ref<{ ok: boolean; text: string } | null>(null);
const copied = ref<'code' | 'link' | null>(null);
const canSend = computed(() => text.value.replace(/[^A-Za-z0-9]/g, '').length >= 8 && !busy.value);

async function copy(what: 'code' | 'link'): Promise<void> {
  const value = what === 'code' ? social.hub.code : social.hub.friendLink;
  if (!value || !(await copyText(value))) return;
  copied.value = what;
  setTimeout(() => (copied.value = null), 2000);
}

async function send(): Promise<void> {
  busy.value = true;
  result.value = null;
  try {
    const r = await addFriend(text.value);
    const key = r.theyAsked ? 'addFriend.theyAsked' : 'addFriend.sent';
    result.value = { ok: true, text: t(key, { name: r.user.name }) };
    text.value = '';
  } catch (e) {
    result.value = { ok: false, text: isRpcError(e) ? e.message : t('connect.genericError') };
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <UiModal :title="$t('addFriend.title')" @close="emit('close')">
    <p v-if="!social.hub.configured" class="note">{{ $t('addFriend.noHub') }}</p>

    <template v-else>
      <section class="mine">
        <span class="label">{{ $t('addFriend.yourCode') }}</span>
        <div class="code-row">
          <strong class="code">{{ social.hub.code }}</strong>
          <UiButton variant="secondary" :size="32" @click="copy('code')">
            <Check v-if="copied === 'code'" :size="16" :stroke-width="1.75" aria-hidden="true" />
            <Copy v-else :size="16" :stroke-width="1.75" aria-hidden="true" />
            {{ copied === 'code' ? $t('invite.copied') : $t('addFriend.copyCode') }}
          </UiButton>
          <UiButton variant="ghost" :size="32" @click="copy('link')">
            <Check v-if="copied === 'link'" :size="16" :stroke-width="1.75" aria-hidden="true" />
            <Link2 v-else :size="16" :stroke-width="1.75" aria-hidden="true" />
            {{ copied === 'link' ? $t('invite.copied') : $t('addFriend.copyLink') }}
          </UiButton>
        </div>
        <p class="note">{{ $t('addFriend.shareHint') }}</p>
      </section>

      <form class="send" @submit.prevent="canSend && send()">
        <UiField v-model="text" :label="$t('addFriend.theirCode')" :placeholder="$t('addFriend.placeholder')" />
        <UiButton variant="primary" type="submit" :disabled="!canSend">
          <Send :size="16" :stroke-width="1.75" aria-hidden="true" />
          {{ busy ? $t('addFriend.sending') : $t('addFriend.send') }}
        </UiButton>
      </form>
      <p v-if="result" :class="result.ok ? 'ok' : 'error'" role="status">{{ result.text }}</p>

      <template v-if="social.friendRequests.incoming.length">
        <span class="label">{{ $t('addFriend.incoming') }}</span>
        <FriendRequests :requests="social.friendRequests.incoming" />
      </template>
      <template v-if="social.friendRequests.outgoing.length">
        <span class="label">{{ $t('addFriend.outgoing') }}</span>
        <FriendRequests :requests="social.friendRequests.outgoing" outgoing />
      </template>
    </template>
  </UiModal>
</template>

<style lang="scss" scoped>
.label {
  @include label;
}

.mine {
  display: grid;
  gap: $sp-2;

  .code-row {
    display: flex;
    align-items: center;
    gap: $sp-2;

    .code {
      margin-right: auto;
      font-size: $fs-h1;
      line-height: $lh-h1;
      letter-spacing: 0.06em;
      font-variant-numeric: tabular-nums;
      user-select: all;
    }
  }
}

.send {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: end;
  gap: $sp-2;
}

.note {
  @include message;
}

.ok {
  @include message($ok);
}

.error {
  @include message($danger);
}
</style>
