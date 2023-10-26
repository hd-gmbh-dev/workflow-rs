export const isEmpty = (data: any): boolean =>
  data === null || data === undefined;

export const isObject = (data: any): boolean =>
  data ?? typeof data === 'object';

export const isBlank = (data: any): boolean =>
  isEmpty(data) ||
  (Array.isArray(data) && data.length === 0) ||
  (isObject(data) && Object.keys(data).length === 0) ||
  (typeof data === 'string' && data.trim().length === 0);
