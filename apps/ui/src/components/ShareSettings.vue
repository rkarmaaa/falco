<script setup lang="ts">
// Qualità della condivisione: quattro profili pronti, uno personalizzato e
// «Trova le impostazioni migliori», che misura la rete e consiglia il profilo.
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Gauge, Scale, Snail, Sparkles, SlidersHorizontal, Wand2, Check, Volume2, Wifi, ShieldCheck, Image, Users, UserCheck } from 'lucide-vue-next';
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
// Audio condiviso: solo le app a schermo (mai le chat vocali) o tutto il PC. Solo Windows (ADR 0016).
const showAudio = !navigator.userAgent.includes('Mac');
const audio = computed({
  get: () => String(core.config?.host.audio_capture ?? 'onscreen') as 'onscreen' | 'all',
  set: (v) => void save('host.audio_capture', v),
});
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

// Scelte a pulsante invece degli slider: pochi valori sensati, ognuno spiegato
const caps = [0, 10_000, 20_000, 40_000, 80_000].map((v) => ({ value: String(v), label: v === 0 ? t('share.noCap') : `${v / 1000} Mbps` }));
const capChoice = computed({
  get: () => String(caps.some((c) => Number(c.value) === cap.value) ? cap.value : 0),
  set: (v) => void save('host.max_bitrate_kbps', Number(v)),
});
const fecs = [
  { value: '5', label: t('share.fecLevel.low') },
  { value: '15', label: t('share.fecLevel.medium') },
  { value: '30', label: t('share.fecLevel.high') },
];
const fecChoice = computed({
  get: () => (fec.value <= 8 ? '5' : fec.value <= 20 ? '15' : '30'),
  set: (v) => void save('network.fec_percent', Number(v)),
});
const guests = computed({
  get: () => String(core.config?.host.max_guests ?? 3),
  set: (v) => void save('host.max_guests', Number(v)),
});
const defaultRole = computed({
  get: () => String(core.config?.host.default_role ?? 'spettatore'),
  set: (v) => void save('host.default_role', v),
});
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

    <div v-if="currentProfile === 'custom'" class="options">
      <div class="option">
        <span class="icon"><Wifi :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
        <div class="what">
          <span class="title">{{ t('share.cap') }}</span>
          <span class="hint">{{ t('share.capHint') }}</span>
        </div>
        <UiSegmented v-model="capChoice" :label="t('share.cap')" :options="caps" />
      </div>
      <div class="option">
        <span class="icon"><ShieldCheck :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
        <div class="what">
          <span class="title">{{ t('share.fec') }}</span>
          <span class="hint">{{ t('share.fecHint') }}</span>
        </div>
        <UiSegmented v-model="fecChoice" :label="t('share.fec')" :options="fecs" />
      </div>
      <div class="option">
        <span class="icon"><Image :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
        <div class="what">
          <span class="title">{{ t('share.encoder') }}</span>
          <span class="hint">{{ t(`share.qualityHint.${quality}`) }}</span>
        </div>
        <UiSegmented
          v-model="quality"
          :label="t('share.encoder')"
          :options="(['speed', 'balanced', 'quality'] as const).map((v) => ({ value: v, label: t(`share.quality.${v}`) }))"
        />
      </div>
    </div>

    <div class="options">
      <div class="option">
        <span class="icon"><Users :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
        <div class="what">
          <span class="title">{{ t('share.guests') }}</span>
          <span class="hint">{{ t('share.guestsHint') }}</span>
        </div>
        <UiSegmented v-model="guests" :label="t('share.guests')" :options="['1', '2', '3', '4'].map((v) => ({ value: v, label: v }))" />
      </div>
      <div class="option">
        <span class="icon"><UserCheck :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
        <div class="what">
          <span class="title">{{ t('share.defaultRole') }}</span>
          <span class="hint">{{ t('share.defaultRoleHint') }}</span>
        </div>
        <UiSegmented
          v-model="defaultRole"
          :label="t('share.defaultRole')"
          :options="(['spettatore', 'giocatore'] as const).map((v) => ({ value: v, label: t(`role.${v}`) }))"
        />
      </div>
      <div v-if="showAudio" class="option">
        <span class="icon"><Volume2 :size="18" :stroke-width="1.8" aria-hidden="true" /></span>
        <div class="what">
          <span class="title">{{ t('share.audio.label') }}</span>
          <span class="hint">{{ t(`share.audio.${audio}Hint`) }}</span>
        </div>
        <UiSegmented
          v-model="audio"
          :label="t('share.audio.label')"
          :options="(['onscreen', 'all'] as const).map((v) => ({ value: v, label: t(`share.audio.${v}`) }))"
        />
      </div>
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

  .options {
    display: grid;
    border: 1px solid $line;
    border-radius: $r-card;
    background: $bg-2;

    .option {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr) auto;
      align-items: center;
      gap: $sp-4;
      padding: $sp-4 $sp-5;

      & + .option {
        border-top: 1px solid $line;
      }

      // Finestra stretta: le scelte vanno sotto la descrizione
      @include mq(md) {
        grid-template-columns: auto minmax(0, 1fr);

        > :last-child {
          grid-column: 2;
          justify-self: start;
        }
      }
    }

    .icon {
      display: grid;
      place-items: center;
      width: 36px;
      height: 36px;
      border-radius: $r-button;
      background: $bg-4;
      color: $accent-fg;
    }

    .what {
      display: grid;
      gap: 2px;
      min-width: 0;
    }

    .title {
      font-weight: $fw-strong;
    }

    .hint {
      @include message($text-3);
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
