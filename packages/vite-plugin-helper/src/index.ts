import jsep from 'jsep';

export function parseJsepExpression(s: string): string {
    return JSON.stringify(jsep(s));
}
