import { describe, expect, it } from '@jest/globals';

import { fromXML } from '../src/parser';

describe('parser', () => {
    it('fromXML', () => {
        expect(() => fromXML('')).toThrow('NoProcessDefinition');
    });
});
