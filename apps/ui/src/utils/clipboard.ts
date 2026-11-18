// Copia negli appunti: plugin di Tauri nella finestra vera, API del browser altrimenti.
import { isTauri } from '@/ipc';

export async function copyText(text: string): Promise<boolean> {
  try {
    if (isTauri()) {
      const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
      await writeText(text);
    } else {
      await navigator.clipboard.writeText(text);
    }
    return true;
  } catch {
    return false;
  }
}
