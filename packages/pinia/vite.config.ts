import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import dts from 'vite-plugin-dts';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

import * as path from 'path';
import typescript2 from 'rollup-plugin-typescript2';

export default defineConfig({
    plugins: [
        wasm(),
        topLevelAwait(),
        vue(),
        dts({
            insertTypesEntry: true,
        }),
        typescript2({
            check: false,
            include: ['src/components/**/*.vue'],
            tsconfigOverride: {
                compilerOptions: {
                    outDir: 'dist',
                    sourceMap: true,
                    declaration: true,
                    declarationMap: true,
                },
            },
            exclude: ['vite.config.ts'],
        }),
    ],
    build: {
        cssCodeSplit: false,
        lib: {
            // Could also be a dictionary or array of multiple entry points
            entry: 'src/index.ts',
            formats: ['es'],
            fileName: (format) => `wfrs-pinia.${format}.js`,
        },
        rollupOptions: {
            // make sure to externalize deps that should not be bundled
            // into your library
            input: {
                main: path.resolve(__dirname, 'src/index.ts'),
            },
            external: [
                'vue',
                'vue-router',
                'pinia',
                'vue-i18n',
                '@wfrs/runtime',
            ],
        },
    },
    resolve: {
        alias: {
            '@': path.resolve(__dirname, 'src'),
        },
    },
});
