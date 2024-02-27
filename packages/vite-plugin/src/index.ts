import fs from 'node:fs';
import fsp from 'node:fs/promises';
import {
    cleanUrl,
    normalizePath,
    withTrailingSlash,
    createToImportMetaURLBasedRelativeRuntime,
    toOutputFilePathInJS,
} from './utils';
import MagicString from 'magic-string';
import path from 'path';
import type {
    NormalizedOutputOptions,
    PluginContext,
    RenderedChunk,
} from 'rollup';
import { fromXML } from './parser';
import type { ResolvedConfig } from 'vite';

const assetUrlRE = /__WFRS_FILE__([a-z\d]+)__(?:\$_(.*?)__)?/g;
const urlRE = /(\?|&)url(?:&|$)/;
const unnededFinalQueryCharRE = /[?&]$/;

const assetCache = new WeakMap<ResolvedConfig, Map<string, string>>();

interface GeneratedAssetMeta {
    originalName: string;
    isEntry?: boolean;
}

const generatedAssets = new WeakMap<
    ResolvedConfig,
    Map<string, GeneratedAssetMeta>
>();

export function checkPublicFile(
    url: string,
    { publicDir }: ResolvedConfig,
): string | undefined {
    // note if the file is in /public, the resolver would have returned it
    // as-is so it's not going to be a fully resolved path.
    if (publicDir === '' || url[0] !== '/') {
        return;
    }
    const publicFile = path.join(publicDir, cleanUrl(url));
    if (
        !normalizePath(publicFile).startsWith(
            withTrailingSlash(normalizePath(publicDir)),
        )
    ) {
        // can happen if URL starts with '../'
        return;
    }
    if (fs.existsSync(publicFile)) {
        return publicFile;
    }
}

const fileRegex = /\.(bpmn)$/;

export function renderAssetUrlInJS(
    ctx: PluginContext,
    config: ResolvedConfig,
    chunk: RenderedChunk,
    opts: NormalizedOutputOptions,
    code: string,
): MagicString | undefined {
    const toRelativeRuntime = createToImportMetaURLBasedRelativeRuntime(
        opts.format,
        config.isWorker,
    );

    let match: RegExpExecArray | null;
    let s: MagicString | undefined;

    assetUrlRE.lastIndex = 0;
    while ((match = assetUrlRE.exec(code) ?? null) !== null) {
        s ??= new MagicString(code);
        const [full, referenceId, postfix = ''] = match;
        const file = ctx.getFileName(referenceId);
        if (chunk.viteMetadata === null) {
            chunk.viteMetadata = {
                importedAssets: new Set([]),
                importedCss: new Set([]),
            };
        }
        chunk.viteMetadata?.importedAssets.add(cleanUrl(file));
        const filename = file + postfix;
        const replacement = toOutputFilePathInJS(
            filename,
            'asset',
            chunk.fileName,
            'js',
            config,
            toRelativeRuntime,
        );
        const replacementString =
            typeof replacement === 'string'
                ? JSON.stringify(replacement).slice(1, -1)
                : `"+${replacement.runtime}+"`;
        s.update(match.index, match.index + full.length, replacementString);
    }
    return s;
}

const PLUGIN_NAME = 'wfrs-vite-plugin';
export default function wfrs(): any {
    let viteConfig: any | null = null;
    return {
        name: PLUGIN_NAME,
        async configResolved(config: any) {
            viteConfig = config;
        },
        buildStart() {
            assetCache.set(viteConfig, new Map());
            generatedAssets.set(viteConfig, new Map());
        },
        async transform(_: string, id: string) {
            if (!fileRegex.test(id)) {
                return;
            }
            id = id.replace(urlRE, '$1').replace(unnededFinalQueryCharRE, '');
            const url = await fileToUrl(
                id,
                id.replace('.bpmn', '.wfrs'),
                viteConfig,
                this,
            );
            return `export default ${JSON.stringify(url)}`;
        },
        renderChunk(code: string, chunk: any, opts: any) {
            const s =
                renderAssetUrlInJS(this, viteConfig, chunk, opts, code) ?? null;
            if (s !== null) {
                const sourcemap = viteConfig.build.sourcemap ?? null;
                return {
                    code: s.toString(),
                    map:
                        sourcemap !== null
                            ? s.generateMap({ hires: 'boundary' })
                            : null,
                };
            } else {
                return null;
            }
        },
    };
}

export async function fileToUrl(
    id: string,
    nextId: string,
    config: ResolvedConfig,
    ctx: any,
): Promise<string> {
    return await fileToBuiltUrl(id, nextId, config, ctx);
}

async function fileToBuiltUrl(
    id: string,
    _nextId: string,
    config: ResolvedConfig,
    _pluginContext: PluginContext,
): Promise<string> {
    const cache = assetCache.get(config) ?? null;
    if (cache !== null) {
        const cached = cache.get(id) ?? null;
        if (cached !== null) {
            return cached;
        }
    }
    const file = cleanUrl(id);
    const fileContent = await fsp.readFile(file, 'utf-8');
    const content = fromXML(fileContent);

    const mimeType = 'application/octet-stream';
    // base64 inlined as a string
    const url = `data:${mimeType};base64,${Buffer.from(content).toString(
        'base64',
    )}`;
    return url;
}
