<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue';
</script>

<template>
  <section class="invites">
    <h1 class="title">{{ $t('invites.title') }}</h1>
    <EmptyState :title="$t('invites.emptyTitle')" :body="$t('invites.emptyBody')" />
  </section>
</template>

<style lang="scss" scoped>
.invites {
  .title {
    font-size: $fs-h1;
    line-height: 30px;
    font-weight: 650;
  }
}
</style>
