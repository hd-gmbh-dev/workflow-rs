import init, {
    WorkflowStore,
    type JsWorkflowDefinition,
    setup,
} from '@wfrs/runtime';
import { type WorkflowSource } from './source';
let wasm: any | null = null;
async function initialize(): Promise<any> {
    wasm = await init();
    setup();
    return wasm;
}

class WorkflowDefinitions {
    initialized: Promise<any> | null = null;
    store: WorkflowStore | null = null;
    loader: Record<string, Promise<JsWorkflowDefinition> | undefined> = {};
    definitions: Record<string, string | undefined> = {};
    taskIds: Record<string, string[]> = {};

    register({ key, url }: WorkflowSource): void {
        this.definitions[key] = url;
    }

    async init(): Promise<void> {
        if (this.initialized == null) {
            this.initialized = initialize();
            await this.initialized;
            this.store = new WorkflowStore();
        } else {
            await this.initialized;
        }
    }

    async load(key: string): Promise<JsWorkflowDefinition | undefined> {
        await this.init();
        const url = this.definitions[key] ?? '';
        const store = this.store ?? null;
        if (url !== '' && store !== null) {
            if ((this.loader[key] ?? null) !== null) {
                const result = await this.loader[key];
                return result;
            } else {
                this.loader[key] = fetch(url)
                    .then(async (r) => await r.arrayBuffer())
                    .then(async (r) => await store.register(new Uint8Array(r)));
                const result = (await this.loader[key]) ?? null;
                if (result !== null) {
                    this.taskIds[key] = result.task_ids().split(',');
                    return result;
                }
            }
        }
    }

    list(): string[] {
        return Object.keys(this.definitions);
    }

    taskId(key: string, idx: number): string | undefined {
        return this.taskIds[key][idx];
    }
}

export default new WorkflowDefinitions();
