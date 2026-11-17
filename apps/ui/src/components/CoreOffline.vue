<script setup lang="ts">
// Errore in tre parti (§13): cosa, perché, cosa fare
import { CircleAlert } from 'lucide-vue-next';
</script>

<template>
  <div class="offline" role="alert">
    <CircleAlert class="icon" :size="18" :stroke-width="1.75" aria-hidden="true" />
    <p>
      <strong>{{ $t('core.offlineTitle') }}</strong>
      {{ $t('core.offlineWhy') }} {{ $t('core.offlineFix') }}
    </p>
  </div>
</template>

<style lang="scss" scoped>
.offline {
  display: flex;
  align-items: flex-start;
  gap: $sp-3;
  margin: $sp-4 $sp-6 0;
  padding: $sp-3 $sp-4;
  border-radius: $r-card;
  border: 1px solid $danger;
  background: $bg-2;
  font-size: $fs-small;
  line-height: 18px;
  color: $text-2;

  .icon {
    flex: none;
    color: $danger;
    margin-top: 1px;
  }

  strong {
    color: $text-1;
  }
}
</style>
