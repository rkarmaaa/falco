<script setup lang="ts">
// Qualità della condivisione: quattro profili pronti, uno personalizzato e
// «Trova le impostazioni migliori», che misura la rete e consiglia il profilo.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Gauge, Scale, Snail, Sparkles, SlidersHorizontal, Wand2, Check } from 'lucide-vue-next';
import UiButton from './UiButton.vue';
import UiSegmented from './UiSegmented.vue';
import { core, hostApi, setConfig } from '@/stores/core';
import { PROFILES, applyProfile, currentProfile, restartIfIdle } from '@/stores/share';
import { attempt } from '@/stores/errors';
import type { HostProfile, Recommendation } from '@/ipc';

const { t } = useI18n();
const icons = { latency: Gauge, balanced: Scale, slow: Snail, quality: Sparkles, custom: SlidersHorizontal };
const note = ref<string | null>(null);
const testing = ref(false);
const advice = ref<Recommendation | null>(null);

const cards = computed(() => [...PROFILES, 'custom' as const]);
const cap = computed(() => Number(core.config?.host.max_bitrate_kbps ?? 0));
const fec = computed(() => Number(core.config?.network.fec_percent ?? 20));
const quality = computed({
  get: () => String(core.config?.host.encoder_quality ?? 'speed') as 'speed' | 'balanced' | 'quality',
  set: (v) => void save('host.encoder_quality', v),
});

function said(r: 'now' | 'later' | 'off' | undefined): void {
  note.value = r === 'now' ? t('share.appliedNow') : r === 'later' ? t('share.appliesLater') : r === 'off' ? t('share.appliesOnStart') : null;
}

async function pick(p: HostProfile): Promise<void> {
  said(await attempt(t('errors.context.profile'), () => applyProfile(p)));
}

async function save(key: string, value: unknown): Promise<void> {
  await attempt(t('errors.context.profile'), async () => {
    await setConfig(key, value);
    if (core.host.state === 'ready') said((await restartIfIdle()) ? 'now' : 'later');
  });
}

async function findBest(): Promise<void> {
  testing.value = true;
  advice.value = null;
  advice.value = (await attempt(t('errors.context.recommend'), () => hostApi.recommend())) ?? null;
  testing.value = false;
}

const mbps = (kbps: number) => (kbps === 0 ? t('share.noCap') : `${Math.round(kbps / 1000)} Mbps`);
</script>

<template>
  <section class="share">
    <header class="head">
      <div>
        <h2>{{ t('share.title') }}</h2>
        <p>{{ t('share.subtitle') }}</p>
      </div>
      <UiButton variant="secondary" :disabled="testing" @click="findBest">
        <Wand2 :size="17" :stroke-width="1.9" aria-hidden="true" />
        {{ testing ? t('share.testing') : t('share.findBest') }}
      </UiButton>
    </header>

    <div v-if="testing" class="advice busy" role="status">
      <span class="bar" aria-hidden="true" />
      {{ t('share.testingBody') }}
    </div>
    <div v-else-if="advice" class="advice" role="status">
      <div class="what">
        <strong>{{ t('share.advice', { profile: t(`share.profiles.${advice.profile}.name`) }) }}</strong>
        <ul>
          <li v-for="r in advice.reasons" :key="r">{{ t(`share.reasons.${r}`) }}</li>
        </ul>
      </div>
      <UiButton v-if="currentProfile !== advice.profile" variant="primary" :size="32" @click="pick(advice.profile)">{{ t('share.apply') }}</UiButton>
      <span v-else class="ok"><Check :size="16" :stroke-width="2.4" aria-hidden="true" />{{ t('share.alreadyBest') }}</span>
    </div>

    <ul class="cards" role="radiogroup" :aria-label="t('share.title')">
      <li v-for="p in cards" :key="p">
        <button type="button" class="card" role="radio" :aria-checked="currentProfile === p" :class="{ active: currentProfile === p }" @click="pick(p)">
          <span class="icon"><component :is="icons[p]" :size="20" :stroke-width="1.8" aria-hidden="true" /></span>
          <span class="name">{{ t(`share.profiles.${p}.name`) }}</span>
          <span class="desc">{{ t(`share.profiles.${p}.desc`) }}</span>
          <span class="tags">{{ t(`share.profiles.${p}.tags`) }}</span>
        </button>
      </li>
    </ul>

    <div v-if="currentProfile === 'custom'" class="custom">
      <label class="range">
        <span class="label">{{ t('share.cap') }} <b>{{ mbps(cap) }}</b></span>
        <input type="range" min="0" max="150000" step="5000" :value="cap" @change="save('host.max_bitrate_kbps', Number(($event.target as HTMLInputElement).value) || 0)" />
        <span class="hint">{{ t('share.capHint') }}</span>
      </label>
      <label class="range">
        <span class="label">{{ t('share.fec') }} <b>{{ fec }}%</b></span>
        <input type="range" min="0" max="50" step="5" :value="fec" @change="save('network.fec_percent', Number(($event.target as HTMLInputElement).value))" />
        <span class="hint">{{ t('share.fecHint') }}</span>
      </label>
      <UiSegmented
        v-model="quality"
        :label="t('share.encoder')"
        :options="(['speed', 'balanced', 'quality'] as const).map((v) => ({ value: v, label: t(`share.quality.${v}`) }))"
      />
    </div>

    <p v-if="note" class="note" role="status">{{ note }}</p>
  </section>
</template>

<style lang="scss" scoped>
.share {
  display: grid;
  gap: $sp-4;

  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: $sp-4;

    h2 {
      font-size: $fs-h2;
      font-weight: 650;
    }

    p {
      margin-top: 2px;
      color: $text-3;
      font-size: $fs-small;
    }
  }

  .advice {
    display: flex;
    align-items: center;
    gap: $sp-4;
    padding: $sp-4;
    border: 1px solid rgba(157, 169, 255, 0.3);
    border-radius: $r-card;
    background: $accent-soft;

    .what {
      flex: 1;

      ul {
        margin: $sp-1 0 0;
        padding-left: $sp-4;
        color: $text-2;
        font-size: $fs-small;
      }
    }

    .ok {
      display: flex;
      align-items: center;
      gap: $sp-1;
      color: $live;
      font-size: $fs-small;
      font-weight: 600;
    }

    &.busy {
      position: relative;
      overflow: hidden;
      color: $text-2;
      font-size: $fs-small;

      .bar {
        position: absolute;
        bottom: 0;
        left: 0;
        width: 40%;
        height: 2px;
        background: $accent-fg;
        animation: slide 1.2s $ease infinite;
      }
    }
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: $sp-3;
    margin: 0;
    padding: 0;
    list-style: none;

    @include mq(md) {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .card {
      @include reset-button;
      @include focus-ring;
      display: grid;
      align-content: start;
      gap: $sp-1;
      width: 100%;
      height: 100%;
      padding: $sp-4;
      border: 1px solid $line;
      border-radius: $r-card;
      background: $bg-2;
      text-align: left;
      transition:
        border-color $t-fast $ease,
        background $t-fast $ease,
        transform $t-fast $ease;

      &:hover {
        border-color: $line-strong;
        background: $bg-3;
      }

      .icon {
        display: grid;
        place-items: center;
        width: 36px;
        height: 36px;
        margin-bottom: $sp-2;
        border-radius: $r-button;
        background: $bg-4;
        color: $text-2;
      }

      .name {
        font-weight: 650;
      }

      .desc {
        color: $text-2;
        font-size: $fs-small;
        line-height: 18px;
      }

      .tags {
        margin-top: $sp-2;
        color: $text-3;
        font-size: 12px;
      }

      &.active {
        border-color: rgba(157, 169, 255, 0.55);
        background: linear-gradient(160deg, rgba(76, 95, 214, 0.22), rgba(19, 22, 38, 0.72) 70%);
        box-shadow: 0 0 0 1px rgba(157, 169, 255, 0.2), 0 12px 32px rgba(50, 64, 154, 0.25);

        .icon {
          background: $grad-accent;
          color: $on-accent;
        }
      }
    }
  }

  .custom {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: $sp-5;
    padding: $sp-5;
    border: 1px solid $line;
    border-radius: $r-card;
    background: $bg-2;

    @include mq(md) {
      grid-template-columns: 1fr;
    }

    .range {
      display: grid;
      gap: $sp-2;

      .label {
        font-size: $fs-small;
        color: $text-2;

        b {
          color: $text-1;
          font-weight: 650;
        }
      }

      .hint {
        color: $text-3;
        font-size: 12px;
      }

      input {
        accent-color: #6e7eff;
      }
    }
  }

  .note {
    color: $accent-fg;
    font-size: $fs-small;
  }
}

@keyframes slide {
  from {
    transform: translateX(-100%);
  }

  to {
    transform: translateX(260%);
  }
}
</style>
