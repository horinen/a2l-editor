import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { join } from 'path';

const projectRoot = join(process.cwd(), '..');

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 5174,
    strictPort: true,
  },
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: false,
    sourcemap: true
  },
  resolve: {
    alias: {
      '@tauri-apps/api/core': join(projectRoot, '.ai-testing/mock-modules/core.js'),
      '@tauri-apps/api/event': join(projectRoot, '.ai-testing/mock-modules/event.js'),
      '@tauri-apps/api/window': join(projectRoot, '.ai-testing/mock-modules/window.js'),
      '@tauri-apps/plugin-dialog': join(projectRoot, '.ai-testing/mock-modules/dialog.js'),
      '@tauri-apps/plugin-clipboard-manager': join(projectRoot, '.ai-testing/mock-modules/clipboard.js'),
    }
  }
});
