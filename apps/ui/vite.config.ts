import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue';
import VueI18nPlugin from '@intlify/unplugin-vue-i18n/vite';

// Porta fissa per Tauri; i messaggi i18n sono precompilati (nessun eval con la CSP)
export default defineConfig({
  plugins: [
    vue(),
    VueI18nPlugin({
      include: [fileURLToPath(new URL('./src/i18n/locales/**', import.meta.url))],
      runtimeOnly: true,
    }),
  ],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  css: {
    preprocessorOptions: {
      scss: {
        additionalData: '@use "@/styles/tokens" as *;\n@use "@/styles/mixins" as *;\n',
      },
    },
  },
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', cssCodeSplit: false, chunkSizeWarningLimit: 300 },
  test: { environment: 'jsdom', globals: false },
});
