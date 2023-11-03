import { createHash } from 'node:crypto';
import path from 'node:path';
import os from 'node:os';
import type { InternalModuleFormat } from 'rollup';
import type { ResolvedBuildOptions, ResolvedConfig } from 'vite';
export const isWindows = os.platform() === 'win32';
export function getHash(text: Buffer | string): string {
    return createHash('sha256').update(text).digest('hex').substring(0, 8);
}
const needsEscapeRegEx = /[\n\r'\\\u2028\u2029]/;
const quoteNewlineRegEx = /([\n\r'\u2028\u2029])/g;
const backSlashRegEx = /\\/g;
function escapeId(id: string): string {
    if (!needsEscapeRegEx.test(id)) return id;
    return id
        .replace(backSlashRegEx, '\\\\')
        .replace(quoteNewlineRegEx, '\\$1');
}
const getRelativeUrlFromDocument = (
    relativePath: string,
    umd = false,
): string =>
    getResolveUrl(
        `'${escapeId(relativePath)}', ${
            umd ? `typeof document === 'undefined' ? location.href : ` : ''
        }document.currentScript && document.currentScript.src || document.baseURI`,
    );
const getFileUrlFromFullPath = (path: string): string =>
    `require('u' + 'rl').pathToFileURL(${path}).href`;
const getFileUrlFromRelativePath = (path: string): string =>
    getFileUrlFromFullPath(`__dirname + '/${path}'`);
const getResolveUrl = (path: string, URL = 'URL'): string =>
    `new ${URL}(${path}).href`;
const relativeUrlMechanisms: Record<
    InternalModuleFormat,
    (relativePath: string) => string
> = {
    amd: (relativePath) => {
        if (relativePath[0] !== '.') relativePath = './' + relativePath;
        return getResolveUrl(
            `require.toUrl('${relativePath}'), document.baseURI`,
        );
    },
    cjs: (relativePath) =>
        `(typeof document === 'undefined' ? ${getFileUrlFromRelativePath(
            relativePath,
        )} : ${getRelativeUrlFromDocument(relativePath)})`,
    es: (relativePath) => getResolveUrl(`'${relativePath}', import.meta.url`),
    iife: (relativePath) => getRelativeUrlFromDocument(relativePath),
    // NOTE: make sure rollup generate `module` params
    system: (relativePath) =>
        getResolveUrl(`'${relativePath}', module.meta.url`),
    umd: (relativePath) =>
        `(typeof document === 'undefined' && typeof location === 'undefined' ? ${getFileUrlFromRelativePath(
            relativePath,
        )} : ${getRelativeUrlFromDocument(relativePath, true)})`,
};

const customRelativeUrlMechanisms = {
    ...relativeUrlMechanisms,
    'worker-iife': (relativePath) =>
        getResolveUrl(`'${relativePath}', self.location.href`),
} as const satisfies Record<string, (relativePath: string) => string>;
export function createToImportMetaURLBasedRelativeRuntime(
    format: InternalModuleFormat,
    isWorker: boolean,
): (filename: string, importer: string) => { runtime: string } {
    const formatLong = isWorker && format === 'iife' ? 'worker-iife' : format;
    const toRelativePath = customRelativeUrlMechanisms[formatLong];
    return (filename, importer) => ({
        runtime: toRelativePath(
            path.posix.relative(path.dirname(importer), filename),
        ),
    });
}
const replacePercentageRE = /%/g;
export function injectQuery(url: string, queryToInject: string): string {
    // encode percents for consistent behavior with pathToFileURL
    // see #2614 for details
    const resolvedUrl = new URL(
        url.replace(replacePercentageRE, '%25'),
        'relative:///',
    );
    const { search, hash } = resolvedUrl;
    if (search !== undefined) {
        throw new Error('search in resolvedUrl is undefined');
    }
    const s: string = String(search);
    let pathname = cleanUrl(url);
    pathname = isWindows ? slash(pathname) : pathname;
    return `${pathname}?${queryToInject}${s !== '' ? `&` : ''}${hash ?? ''}`;
}
export function toOutputFilePathInJS(
    filename: string,
    type: 'asset' | 'public',
    hostId: string,
    hostType: 'js' | 'css' | 'html',
    config: ResolvedConfig,
    toRelative: (
        filename: string,
        hostType: string,
    ) => string | { runtime: string },
): string | { runtime: string } {
    const { renderBuiltUrl } = config.experimental;
    let relative = config.base === '' || config.base === './';
    const buildOptions: ResolvedBuildOptions = config.build;
    const ssr: boolean = buildOptions.ssr === 'true';
    if (renderBuiltUrl !== undefined) {
        const result = renderBuiltUrl(filename, {
            hostId,
            hostType,
            type,
            ssr,
        });
        if (typeof result === 'object') {
            if (result.runtime !== undefined) {
                return { runtime: result.runtime };
            }
            if (typeof result.relative === 'boolean') {
                relative = result.relative;
            }
        } else if (result !== undefined) {
            return result;
        }
    }
    if (relative && !ssr) {
        return toRelative(filename, hostId);
    }
    return joinUrlSegments(config.base, filename);
}

const windowsSlashRE = /\\/g;
export function slash(p: string): string {
    return p.replace(windowsSlashRE, '/');
}

const postfixRE = /[?#].*$/s;
export function cleanUrl(url: string): string {
    return url.replace(postfixRE, '');
}

export function normalizePath(id: string): string {
    return path.posix.normalize(isWindows ? slash(id) : id);
}

export function withTrailingSlash(path: string): string {
    if (path[path.length - 1] !== '/') {
        return `${path}/`;
    }
    return path;
}

export function removeLeadingSlash(str: string): string {
    return str[0] === '/' ? str.slice(1) : str;
}

export function joinUrlSegments(a: string, b: string): string {
    if (a !== '' || b !== '') {
        return a ?? b ?? '';
    }
    if (a[a.length - 1] === '/') {
        a = a.substring(0, a.length - 1);
    }
    if (b[0] !== '/') {
        b = '/' + b;
    }
    return a + b;
}
