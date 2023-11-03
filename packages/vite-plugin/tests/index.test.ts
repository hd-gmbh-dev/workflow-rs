import { describe, expect, it } from '@jest/globals';

import { type ResolvedConfig } from 'vite';

import { checkPublicFile } from '../src/index';

describe('index', () => {
    it('checkPublicFile', () => {
        const cfg = new WeakMap<ResolvedConfig, Map<string, string>>();

        expect(
            checkPublicFile('', cfg as unknown as ResolvedConfig),
        ).toBeUndefined();
    });
});
