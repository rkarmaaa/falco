<script setup lang="ts">
// Controlli generati dallo schema di config.toml (§15): ogni chiave con il suo momento di applicazione
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import UiSwitch from './UiSwitch.vue';
import { core, setConfig } from '@/stores/core';
import { isRpcError, rpc } from '@/ipc';

interface Field {
  key: string;
  type: 'bool' | 'int' | 'enum' | 'autoInt' | 'text';
  min?: number;
  max?: number;
  options?: (string | number)[];
  applyAt: 'now' | 'nextSession' | 'hostRestart';
}

const props = defineProps<{ sections: string[] }>();
const emit = defineEmits<{ saved: [] }>();
const { t, te } = useI18n();
const schema = ref<Field[]>([]);
const errors = ref<Record<string, string>>({});

const fields = computed(() => schema.value.filter((f) => props.sections.some((s) => f.key.startsWith(`${s}.`))));
const label = (key: string) => (te(`cfg.${key}`) ? t(`cfg.${key}`) : key);
const option = (_key: string, v: string | number) => (te(`cfg.opt.${v}`) ? t(`cfg.opt.${v}`) : String(v));

function value(key: string): unknown {
  const [section, name] = key.split('.') as [string, string];
  return (core.config as unknown as Record<string, Record<string, unknown>> | null)?.[section]?.[name];
}

async function save(key: string, v: unknown): Promise<void> {
  delete errors.value[key];
  try {
    await setConfig(key, v);
    emit('saved');
  } catch (e) {
    errors.value[key] = isRpcError(e) ? e.message : t('connect.genericError');
  }
}

onMounted(async () => {
  schema.value = await rpc<Field[]>('config.schema');
});
</script>

<template>
  <div class="form">
    <div v-for="f in fields" :key="f.key" class="row">
      <template v-if="f.type === 'bool'">
        <UiSwitch :model-value="Boolean(value(f.key))" :label="label(f.key)" @update:model-value="save(f.key, $event)" />
      </template>
      <template v-else>
        <label class="field">
          <span class="name">{{ label(f.key) }}</span>
          <select
            v-if="f.type === 'enum' || f.type === 'autoInt'"
            :value="String(value(f.key))"
            @change="
              save(
                f.key,
                f.type === 'autoInt' && ($event.target as HTMLSelectElement).value !== 'auto'
                  ? Number(($event.target as HTMLSelectElement).value)
                  : ($event.target as HTMLSelectElement).value,
              )
            "
          >
            <option v-if="f.type === 'autoInt'" value="auto">{{ option(f.key, 'auto') }}</option>
            <option v-for="o in f.options" :key="o" :value="String(o)">{{ option(f.key, o) }}</option>
          </select>
          <input
            v-else-if="f.type === 'int'"
            type="number"
            :min="f.min"
            :max="f.max"
            :value="value(f.key)"
            @change="save(f.key, Number(($event.target as HTMLInputElement).value))"
          />
          <input v-else type="text" :value="value(f.key)" @change="save(f.key, ($event.target as HTMLInputElement).value)" />
        </label>
      </template>
      <span class="when" :title="$t('settings.applyAtHint')">{{ $t(`settings.applyAt.${f.applyAt}`) }}</span>
      <p v-if="errors[f.key]" class="error" role="alert">{{ errors[f.key] }}</p>
    </div>
  </div>
</template>

<style lang="scss" scoped>
.form {
  display: grid;
  gap: $sp-2;

  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: $sp-1 $sp-4;
    padding: $sp-1 0;
    border-bottom: 1px solid $line;

    .field {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: $sp-4;
      min-height: $control-h;

      .name {
        font-weight: 600;
      }

      select,
      input {
        @include surface(2);
        @include focus-ring;
        height: 32px;
        min-width: 160px;
        padding: 0 $sp-2;
        border-radius: $r-field;
        color: $text-1;
        font: inherit;
      }
    }

    .when {
      font-size: $fs-label;
      color: $text-3;
      white-space: nowrap;
    }

    .error {
      grid-column: 1 / -1;
      font-size: $fs-small;
      color: $danger;
    }
  }
}
</style>
