import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  base: './',
  plugins: [react()],
  server: { fs: { allow: ['..'] } },
  build: {
    target: 'es2020',
    assetsInlineLimit: 0,
    cssCodeSplit: false,
    rollupOptions: { output: { inlineDynamicImports: true, entryFileNames: 'app.js', assetFileNames: 'app.[ext]' } },
  },
  worker: { format: 'es' },
});
