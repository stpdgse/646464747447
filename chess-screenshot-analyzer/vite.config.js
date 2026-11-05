import { defineConfig } from 'vite';

export default defineConfig({
  base: './', // works from any sub-path / static host
  build: { target: 'es2020', outDir: 'dist', emptyOutDir: true },
  server: { host: true },
});
