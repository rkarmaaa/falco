<script setup lang="ts">
// Avvisi: richieste (bussa, inviti a giocare, richieste di amicizia), errori con Copia/Debug
// e messaggi brevi. Sopra il palco dello stream si affiancano invece di coprirlo.
import UiButton from './UiButton.vue';
import UiAvatar from './UiAvatar.vue';
import ErrorNotice from './ErrorNotice.vue';
import { answerKnock, answerRequest, declineInvite, dismissPlayInvite, social } from '@/stores/social';
import { acceptDirectInvite } from '@/stores/ui';
import { joinFriend } from '@/stores/core';
import { attempt, dismiss, errors } from '@/stores/errors';
import { useI18n } from 'vue-i18n';

export interface Toast {
  text: string;
  action?: { label: string; run: () => void };
}

defineProps<{ toast: Toast | null; docked?: boolean }>();
const emit = defineEmits<{ clear: [] }>();
const { t } = useI18n();

function friend(id: string) {
  return social.friends.find((f) => f.id === id);
}

function join(id: string, name: string): void {
  dismissPlayInvite(id);
  void attempt(t('errors.context.join', { name }), () => joinFriend(id, name), { name });
}
</script>

<template>
  <div class="toasts" :class="{ docked }" aria-live="polite">
    <div v-for="k in social.knocks" :key="`k${k.friendId}`" class="toast request" role="alert">
      <UiAvatar :name="k.name" :src="friend(k.friendId)?.avatarUrl" :accent="friend(k.friendId)?.accent" :size="32" />
      <span class="text">{{ t('knock.wants', { name: k.name }) }}</span>
      <div class="acts">
        <UiButton variant="ghost" :size="32" @click="attempt(t('errors.context.knock'), () => answerKnock(k.friendId, false))">{{ t('requests.decline') }}</UiButton>
        <UiButton variant="primary" :size="32" @click="attempt(t('errors.context.knock'), () => answerKnock(k.friendId, true))">{{ t('requests.accept') }}</UiButton>
      </div>
    </div>
    <div v-for="p in social.playInvites" :key="`p${p.friendId}`" class="toast request" role="alert">
      <UiAvatar :name="p.name" :src="friend(p.friendId)?.avatarUrl" :accent="friend(p.friendId)?.accent" :size="32" />
      <span class="text">{{ t('knock.invited', { name: p.name }) }}</span>
      <div class="acts">
        <UiButton variant="ghost" :size="32" @click="dismissPlayInvite(p.friendId)">{{ t('knock.later') }}</UiButton>
        <UiButton variant="primary" :size="32" @click="join(p.friendId, p.name)">{{ t('friends.join') }}</UiButton>
      </div>
    </div>
    <div v-for="i in social.friendRequests.invites" :key="`h${i.id}`" class="toast request" role="alert">
      <UiAvatar :name="i.user.name" :size="32" />
      <span class="text">{{ t('knock.invited', { name: i.user.name }) }}</span>
      <div class="acts">
        <UiButton variant="ghost" :size="32" @click="attempt(t('errors.context.request'), () => declineInvite(i.id))">{{ t('requests.decline') }}</UiButton>
        <UiButton variant="primary" :size="32" @click="acceptDirectInvite(i.id, i.user.name)">{{ t('friends.join') }}</UiButton>
      </div>
    </div>
    <div v-for="r in social.requests" :key="`r${r.requestId}`" class="toast request" role="alert">
      <UiAvatar :name="r.name" :size="32" />
      <span class="text">{{ t('requests.wants', { name: r.name }) }} <span class="tag">{{ r.tag }}</span></span>
      <div class="acts">
        <UiButton variant="ghost" :size="32" @click="attempt(t('errors.context.request'), () => answerRequest(r.requestId, false))">{{ t('requests.decline') }}</UiButton>
        <UiButton variant="primary" :size="32" @click="attempt(t('errors.context.request'), () => answerRequest(r.requestId, true))">{{ t('requests.accept') }}</UiButton>
      </div>
    </div>

    <TransitionGroup name="toast">
      <ErrorNotice v-for="e in errors.toasts" :key="e.id" floating :error="e" dismissible @dismiss="dismiss(e.id)" />
      <div v-if="toast" key="msg" class="toast">
        <span class="text">{{ toast.text }}</span>
        <UiButton
          v-if="toast.action"
          variant="secondary"
          :size="32"
          @click="
            toast.action.run();
            emit('clear');
          "
        >
          {{ toast.action.label }}
        </UiButton>
      </div>
    </TransitionGroup>
  </div>
</template>

<style lang="scss" scoped>
.toasts {
  position: fixed;
  z-index: 40;
  right: $sp-5;
  bottom: calc(#{$actionbar-h} + #{$sp-4});
  display: grid;
  gap: $sp-2;
  justify-items: end;
  width: min(420px, calc(100vw - 32px));
  pointer-events: none;

  // Accanto al palco: nel flusso della pagina, non sopra lo stream
  &.docked {
    position: static;
    align-content: end;
    width: 340px;
    height: 100%;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: $sp-3;
    width: 100%;
    padding: $sp-3 $sp-3 $sp-3 $sp-4;
    @include floating-card;

    .text {
      flex: 1;
      min-width: 0;
      font-size: $fs-small;
      line-height: $lh-small;
    }

    .tag {
      color: $text-3;
    }

    .acts {
      display: flex;
      gap: $sp-1;
    }

    &.request {
      flex-wrap: wrap;

      .acts {
        margin-left: auto;
      }
    }
  }
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity $t-panel $ease,
    transform $t-panel $ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
