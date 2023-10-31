import { init, parse } from '@wfrs/parser';

export interface ParseResult {
    errors: string[];
    result: Uint8Array;
}

export function fromXML(src: string): Uint8Array {
    init();
    return parse(src);
}
