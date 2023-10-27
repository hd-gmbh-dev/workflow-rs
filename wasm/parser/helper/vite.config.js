import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    target: 'es2021',
    lib: {
      formats: ['cjs'],
      name: 'helper',
      entry: './src/main.js',
    },
  },
});
