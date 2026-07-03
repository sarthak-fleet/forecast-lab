import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react-swc';
import tailwindcss from '@tailwindcss/vite';

// Fleet standard (VoidZero ecosystem): Vite + React SPA, Tailwind v4 via the
// official @tailwindcss/vite plugin, Lightning CSS as the CSS transformer +
// minifier. Reference: today-little-log.
export default defineConfig({
  server: { host: '::', port: 5173 },
  plugins: [react(), tailwindcss()],
  css: {
    transformer: 'lightningcss',
    lightningcss: { drafts: { customMedia: true } },
  },
  build: {
    modulePreload: false,
    cssMinify: 'lightningcss',
  },
});
