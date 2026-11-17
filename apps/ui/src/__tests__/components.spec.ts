import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { i18n } from '@/i18n';
import UiAvatar from '@/components/UiAvatar.vue';
import UiSwitch from '@/components/UiSwitch.vue';
import EmptyState from '@/components/EmptyState.vue';
import DiagnosticRow from '@/components/DiagnosticRow.vue';

const global = { plugins: [i18n] };

describe('UiAvatar', () => {
  it('mostra il monogramma senza immagine', () => {
    const w = mount(UiAvatar, { props: { name: 'marco rossi', status: 'playing' } });
    expect(w.text()).toBe('MR');
    expect(w.classes()).toContain('playing');
    expect(w.attributes('aria-label')).toBe('marco rossi');
  });
});

describe('UiSwitch', () => {
  it('è un role=switch che cambia al clic', async () => {
    const w = mount(UiSwitch, { props: { modelValue: false, label: 'Disponibile' } });
    const sw = w.get('[role="switch"]');
    expect(sw.attributes('aria-checked')).toBe('false');
    await sw.trigger('click');
    expect(w.emitted('update:modelValue')?.[0]).toEqual([true]);
  });
});

describe('EmptyState', () => {
  it('rende titolo, testo e azione', () => {
    const w = mount(EmptyState, {
      props: { title: 'Qui saranno i tuoi amici', body: 'Invita qualcuno' },
      slots: { default: '<button>Invita il primo</button>' },
    });
    expect(w.get('h2').text()).toBe('Qui saranno i tuoi amici');
    expect(w.text()).toContain('Invita il primo');
  });
});

describe('DiagnosticRow', () => {
  it('indica lo stato anche con il testo, non solo col colore', () => {
    const w = mount(DiagnosticRow, {
      global,
      props: { check: { id: 'tailscaleInstalled', level: 'fail', value: null, action: 'installTailscale' } },
    });
    expect(w.text()).toContain('Tailscale installato');
    expect(w.text()).toContain('Problema');
    expect(w.text()).toContain('Installa Tailscale');
  });
});
