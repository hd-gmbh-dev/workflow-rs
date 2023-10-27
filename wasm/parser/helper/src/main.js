import jsep from 'jsep';

export function parse_jsep_expression(s) {
  return JSON.stringify(jsep(s));
}
