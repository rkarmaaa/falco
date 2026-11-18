// Profili di condivisione dell'host: bitrate massimo, FEC e qualità dell'encoder (ADR 0014).
import { computed } from 'vue';
import { core, setConfig, setHostAvailable } from './core';
import { social } from './social';
import type { HostProfile } from '@/ipc';

export const PROFILES: Exclude<HostProfile, 'custom'>[] = ['latency', 'balanced', 'slow', 'quality'];

export const currentProfile = computed<HostProfile>(() => (core.config?.host.profile as HostProfile | undefined) ?? 'latency');

/** Riavvia la condivisione per applicare le modifiche, se nessuno è collegato. Ritorna se l'ha fatto. */
export async function restartIfIdle(): Promise<boolean> {
  if (core.host.state !== 'ready' || social.guests.length > 0) return false;
  await setHostAvailable(false);
  await setHostAvailable(true);
  return true;
}

/** Sceglie un profilo; con la condivisione attiva e nessun ospite si applica subito. */
export async function applyProfile(p: HostProfile): Promise<'now' | 'later' | 'off'> {
  await setConfig('host.profile', p);
  if (core.host.state !== 'ready') return 'off';
  return (await restartIfIdle()) ? 'now' : 'later';
}
