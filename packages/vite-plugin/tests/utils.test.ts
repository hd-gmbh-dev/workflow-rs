import { describe, expect, it } from '@jest/globals';

import { joinUrlSegments } from '../src/utils';

describe('utils', () => {
    it('joinUrlSegments', () => {
        expect(joinUrlSegments('', '')).toStrictEqual('/');
    });
});
