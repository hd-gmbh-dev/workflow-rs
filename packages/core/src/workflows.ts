// TODO: fix eslint errors for the following rules
/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable @typescript-eslint/no-dynamic-delete */
/* eslint-disable @typescript-eslint/no-misused-promises */
/* eslint-disable @typescript-eslint/no-non-null-assertion */
/* eslint-disable @typescript-eslint/prefer-nullish-coalescing */
/* eslint-disable @typescript-eslint/strict-boolean-expressions */
import WorkflowDefinitions from './definitions';
import { WorkflowSource } from './source';
import {
    CompleteRequest,
    CreateRequest,
    IsSyncRequest,
    ListenRequest,
    LoadRequest,
    UpdateRequest,
} from './proto/wfrs';
import { GrpcWebFetchTransport } from '@protobuf-ts/grpcweb-transport';
import {
    type RpcOptions,
    type UnaryCall,
    type ServerStreamingCall,
} from '@protobuf-ts/runtime-rpc';
import { Subject } from 'rxjs';
import { WorkflowClient, type IWorkflowClient } from './proto/wfrs.client';
import { JsWorkflowDefinition, JsWorkflowInstance } from '@wfrs/runtime';
import { WorkflowEventType, type WorkflowEvent } from './events';

export { WorkflowDefinitions, WorkflowSource };
export { JsWorkflowDefinition, JsWorkflowInstance };

export type StringMap = Record<string, string>;

export type AnyMap = Record<string, any>;

export interface Route {
    path: string;
    params: StringMap;
    query: StringMap;
    meta?: AnyMap;
}

export interface Router {
    currentRoute: () => Route;
    replace: (route: Route) => void;
    push: (route: Route | string) => void;
}

export interface WorkflowRsContext {
    route?: () => RouteCtx | undefined;
    router?: Router;
}

export interface TaskInfo {
    n: number;
    id: string;
    visited: boolean;
    current: boolean;
}

export interface Workflow {
    definition: JsWorkflowDefinition | undefined;
    exist: boolean;
    existRemote?: boolean;
    remoteId?: string;
    remoteVersion?: bigint;
    state?: Uint8Array;
    definitionId: string;
    instanceId: string;
    version: string;
    cancelTo?: string;
    completeTo?: string;
}

export interface SessionStore {
    loggedIn: boolean;
    authToken: string;
}
export interface RouteParams {
    instanceId?: string;
}
export interface RouteMeta {
    definitionId?: string;
}
export interface RouteCtx {
    definitionId?: string;
    instanceId: string;
    completeTo?: string;
    cancelTo?: string;
}
export interface AuthCtx {
    token?: string;
    userId?: string;
}
export interface Freeable {
    free: () => void;
}
export interface Destroyable {
    destroy: () => Promise<void>;
}
export type RouteContextProvider = () => RouteCtx;
export type AuthContextProvider = () => AuthCtx;
function defaultRouteContextProvider(): RouteCtx {
    return {
        instanceId: '0',
    };
}
function defaultAuthContextProvider(): AuthCtx {
    return {};
}
export type Option<T> = T | null;
export type UpdateFn<T> = (t: T) => Option<T>;
export class InstanceState<T> {
    data: Record<string, Record<string, Option<T>>> = {};
    routeContextProvider: RouteContextProvider = defaultRouteContextProvider;

    setRouteContextProvider(routeContextProvider: RouteContextProvider): void {
        this.routeContextProvider = routeContextProvider;
    }

    setContextProvider(routeContextProvider: RouteContextProvider): void {
        this.routeContextProvider = routeContextProvider;
    }

    get(): Option<T> {
        const c = this.routeContextProvider();
        return this.getWith(c);
    }

    getWith(c: RouteCtx): Option<T> {
        const definitionId = c.definitionId ?? null;
        const instanceId = c.instanceId ?? null;
        if (definitionId !== null && instanceId !== null) {
            const d = this.data[definitionId] ?? null;
            if (d !== null) {
                const i = d[instanceId] ?? null;
                if (i !== null) {
                    return i;
                }
            }
        }
        return null;
    }

    set(state: T): void {
        const c = this.routeContextProvider();
        this.setWith(c, state);
    }

    setWith(c: RouteCtx, state: T): void {
        const { definitionId, instanceId } = c;
        if (definitionId && instanceId) {
            this.data[definitionId] = this.data[definitionId] || {};
            this.data[definitionId][instanceId] = state;
        }
    }

    update(updateFn: UpdateFn<T>): void {
        const c = this.routeContextProvider();
        this.updateWith(c, updateFn);
    }

    updateWith(c: RouteCtx, updateFn: UpdateFn<T>): void {
        const { definitionId, instanceId } = c;
        if (definitionId && instanceId) {
            if (
                this.data[definitionId] &&
                this.data[definitionId][instanceId]
            ) {
                const nextState: Option<T> = updateFn(
                    this.data[definitionId][instanceId] as T,
                );
                if (nextState) {
                    this.data[definitionId][instanceId] = nextState;
                }
            }
        }
    }

    async delete(): Promise<void> {
        const c = this.routeContextProvider();
        await this.deleteWith(c);
    }

    async deleteWith(c: RouteCtx): Promise<void> {
        const { definitionId, instanceId } = c;
        if (definitionId && instanceId) {
            if (
                this.data[definitionId] &&
                this.data[definitionId][instanceId]
            ) {
                if (
                    (this.data[definitionId][instanceId] as Destroyable).destroy
                ) {
                    const destroyable = this.data[definitionId][
                        instanceId
                    ] as Destroyable;
                    await destroyable.destroy();
                } else if (
                    (this.data[definitionId][instanceId] as Freeable).free
                ) {
                    const freeable = this.data[definitionId][
                        instanceId
                    ] as Freeable;
                    freeable.free();
                }
                this.data[definitionId][instanceId] = null;
                delete this.data[definitionId][instanceId];
            }
        }
    }
}

interface WorkflowContextProvider {
    routeContextProvider: RouteContextProvider;
    authContextProvider: AuthContextProvider;
}

function defaultContextProvider(): WorkflowContextProvider {
    return {
        routeContextProvider: defaultRouteContextProvider,
        authContextProvider: defaultAuthContextProvider,
    };
}

export class WorkflowRs {
    ignoreNextUpdate: boolean = false;
    ignoreNextComplete: boolean = false;
    instances = new InstanceState<JsWorkflowInstance | undefined>();

    variables = new InstanceState<Record<number, object>>();
    taskInfoLists = new InstanceState<TaskInfo[]>();
    events = new Subject<WorkflowEvent>();
    client: IWorkflowClient | undefined;
    abortController: AbortController | undefined;
    context: WorkflowContextProvider = defaultContextProvider();

    notify(event: WorkflowEvent): void {
        this.events.next(event);
    }

    setContext(context: WorkflowContextProvider): void {
        this.instances.setRouteContextProvider(context.routeContextProvider);
        this.variables.setRouteContextProvider(context.routeContextProvider);
        this.taskInfoLists.setRouteContextProvider(
            context.routeContextProvider,
        );
        this.context = context;
        const transport = new GrpcWebFetchTransport({
            baseUrl: '',
            format: 'binary',
            interceptors: [
                {
                    // adds auth header to unary requests
                    interceptServerStreaming(
                        next,
                        method,
                        input,
                        options,
                    ): ServerStreamingCall {
                        if (!options.meta) {
                            options.meta = {};
                        }
                        if (context.authContextProvider().token) {
                            options.meta.Authorization = `Bearer ${
                                context.authContextProvider().token
                            }`;
                        }
                        return next(method, input, options);
                    },
                    interceptUnary(
                        next,
                        method,
                        input,
                        options: RpcOptions,
                    ): UnaryCall {
                        if (!options.meta) {
                            options.meta = {};
                        }
                        if (context.authContextProvider().token) {
                            options.meta.Authorization = `Bearer ${
                                context.authContextProvider().token
                            }`;
                        }
                        return next(method, input, options);
                    },
                },
            ],
        });
        this.client = new WorkflowClient(transport);
    }

    register(definitions: Iterable<WorkflowSource>): string[] {
        const result: string[] = [];
        for (const definition of definitions) {
            WorkflowDefinitions.register({
                key: definition.key,
                url: definition.url,
            });
            result.push(definition.key);
        }
        return result;
    }

    async load(): Promise<Workflow | undefined> {
        const { definitionId, instanceId, cancelTo, completeTo } =
            this.context.routeContextProvider();
        const { userId } = this.context.authContextProvider();
        if (definitionId && userId) {
            this.notify({
                type: WorkflowEventType.LoadDefinition,
                payload: { id: definitionId, version: '' },
            });
            const result = await this.loadWith(
                definitionId,
                instanceId,
                cancelTo,
                completeTo,
            );
            if (!result.exist) {
                const remote = await this.client?.load(
                    LoadRequest.create({
                        ctx: result.instanceId,
                        def: result.definitionId,
                        ver: result.version,
                    }),
                );
                if (remote?.response.workflow) {
                    result.existRemote = true;
                    result.state = remote.response.workflow.state;
                    result.remoteId = remote.response.workflow.key?.id;
                    result.remoteVersion = BigInt(remote.response.workflow.ts);
                }
            }
            this.notify({
                type: WorkflowEventType.LoadedDefinition,
                payload: { id: definitionId, version: result.version },
            });
            return result;
        }
    }

    async loadWith(
        definitionId: string,
        instanceId: string,
        cancelTo?: string,
        completeTo?: string,
    ): Promise<Workflow> {
        const definition = await WorkflowDefinitions.load(definitionId);
        const version = definition?.version() || '';
        let exist = await definition?.exist(instanceId);
        if (exist) {
            this.notify({
                type: WorkflowEventType.LoadInstance,
                payload: null,
            });
            const instance = await definition?.load(instanceId);
            if (instance) {
                const remote_id = await instance?.get_remote_id();
                if (remote_id) {
                    const synced = await this.client?.isSync(
                        IsSyncRequest.create({
                            key: { id: remote_id.id() },
                            ts: remote_id.ts.toString(),
                        }),
                    );
                    if (!synced?.response.sync) {
                        exist = false;
                        await instance.destroy();
                    } else {
                        this.instances.setWith(
                            { definitionId, instanceId },
                            instance,
                        );
                        await this.instanceUpdate(
                            WorkflowEventType.LoadedInstance,
                            remote_id.id(),
                            instance,
                            remote_id.ts.toString(),
                        );
                        return {
                            definition,
                            exist,
                            definitionId,
                            instanceId,
                            version,
                            cancelTo,
                            completeTo,
                        };
                    }
                }
            }
        }
        return {
            definition,
            exist: false,
            definitionId,
            instanceId,
            version,
            cancelTo,
            completeTo,
        };
    }

    async resume(): Promise<void> {
        this.notify({ type: WorkflowEventType.ResumeStart, payload: null });
        const workflow = await this.load();
        if (workflow) {
            if (workflow.exist) {
                await this.restore(workflow);
            } else if (
                workflow.existRemote &&
                workflow.state &&
                workflow.remoteId &&
                workflow.remoteVersion
            ) {
                await this.restoreRemote(workflow);
            } else if (workflow.definition?.has_autostart()) {
                await this.startWith(workflow, true);
            }
            this.notify({ type: WorkflowEventType.ResumeEnd, payload: null });
        } else {
            this.notify({ type: WorkflowEventType.ResumeError, payload: null });
        }

        if (workflow?.remoteId) {
            setTimeout(async () => {
                this.abortController = new AbortController();
                const listener = this.client?.listen(
                    ListenRequest.create({
                        key: { id: workflow?.remoteId },
                    }),
                    { abort: this.abortController.signal },
                );
                listener?.responses.onComplete(() => {
                    console.log('LISTEN: is completed');
                });
                listener?.responses.onError((_) => {});
                listener?.responses.onMessage(async (m) => {
                    let instance;
                    let active;
                    switch (m.event.oneofKind) {
                        case 'updateEvent':
                            if (this.ignoreNextUpdate) {
                                this.ignoreNextUpdate = false;
                                return;
                            }
                            instance = this.instances.getWith(workflow);
                            if (instance) {
                                await instance.set_state(
                                    m.event.updateEvent.state,
                                );
                                active = await instance?.get_active();
                                await this.updateNav(
                                    workflow.definitionId,
                                    workflow.instanceId,
                                    instance,
                                );
                                await this.updateRoute(
                                    workflow.definitionId,
                                    workflow.instanceId,
                                    active,
                                );
                                if (m.event.updateEvent.ts) {
                                    await this.instanceUpdate(
                                        WorkflowEventType.InstanceUpdate,
                                        workflow.remoteId!,
                                        instance,
                                        m.event.updateEvent.ts,
                                    );
                                }
                            }
                            break;
                        case 'completeEvent':
                            if (this.ignoreNextComplete) {
                                this.ignoreNextComplete = false;
                                return;
                            }
                            instance = this.instances.getWith(workflow);
                            if (instance) {
                                await this.instances.deleteWith(workflow);
                                await this.variables.deleteWith(workflow);
                                await this.taskInfoLists.deleteWith(workflow);
                                if (
                                    m.event.completeEvent &&
                                    m.event.completeEvent.cancelled
                                ) {
                                    this.notify({
                                        type: WorkflowEventType.Cancelled,
                                        payload: {
                                            cancelTo: workflow.cancelTo,
                                            definitionId: workflow.definitionId,
                                            instanceId: workflow.instanceId,
                                        },
                                    });
                                } else {
                                    this.notify({
                                        type: WorkflowEventType.Completed,
                                        payload: {
                                            completeTo: workflow.completeTo,
                                            definitionId: workflow.definitionId,
                                            instanceId: workflow.instanceId,
                                        },
                                    });
                                }
                            }
                            break;
                        default:
                            break;
                    }
                });
                try {
                    await listener;
                } catch (_) {}
            }, 0);
        }
    }

    leave(): void {
        if (this.abortController) {
            this.abortController.abort();
            this.abortController = undefined;
        }
    }

    async restore(workflow: Workflow): Promise<void> {
        const instance = this.instances.getWith({
            definitionId: workflow.definitionId,
            instanceId: workflow.instanceId,
        });
        const active = await instance?.get_active();
        if (active !== undefined && instance) {
            const remoteId = await instance.get_remote_id();
            if (remoteId) {
                workflow.remoteId = remoteId.id();
            }
            await this.updateRoute(
                workflow.definitionId,
                workflow.instanceId,
                active,
                true,
            );
            await this.updateNav(
                workflow.definitionId,
                workflow.instanceId,
                instance,
            );
        }
    }

    async restoreRemote(workflow: Workflow): Promise<void> {
        if (workflow.remoteId && workflow.remoteVersion && workflow.state) {
            const instance = await workflow.definition?.restore(
                workflow.instanceId,
                workflow.remoteId,
                workflow.remoteVersion,
                workflow.state,
            );
            const active = await instance?.get_active();
            if (active !== undefined) {
                this.instances.setWith(
                    {
                        definitionId: workflow.definitionId,
                        instanceId: workflow.instanceId,
                    },
                    instance,
                );
                await this.updateRoute(
                    workflow.definitionId,
                    workflow.instanceId,
                    active,
                    true,
                );
                if (instance) {
                    await this.updateNav(
                        workflow.definitionId,
                        workflow.instanceId,
                        instance,
                    );
                    await this.instanceUpdate(
                        WorkflowEventType.LoadedInstance,
                        workflow.remoteId,
                        instance,
                        workflow.remoteVersion.toString(),
                    );
                }
            }
        }
    }

    async complete(): Promise<void> {
        const c = this.context.routeContextProvider();
        if (c) {
            const { definitionId, instanceId } = c;
            if (definitionId && instanceId) {
                const instance = this.instances.getWith(c);
                if (instance) {
                    const active = await instance?.get_active();
                    if (active !== undefined) {
                        await instance.complete(active);
                        const isCompleted = await instance.is_completed();
                        const remote_id = await instance.get_remote_id();
                        const state = await instance.state();
                        if (remote_id && state) {
                            if (isCompleted) {
                                await this.cancel(true);
                            } else {
                                const id = remote_id.id();
                                const result = await this.client?.update(
                                    UpdateRequest.create({
                                        key: { id },
                                        state,
                                    }),
                                );
                                if (result?.response.ts) {
                                    await this.instanceUpdate(
                                        WorkflowEventType.InstanceUpdate,
                                        id,
                                        instance,
                                        result?.response.ts,
                                    );
                                }
                                const nextActive = await instance.get_active();
                                if (nextActive !== undefined) {
                                    await this.updateRoute(
                                        definitionId,
                                        instanceId,
                                        nextActive,
                                    );
                                }
                                await this.updateNav(
                                    definitionId,
                                    instanceId,
                                    instance,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    async instanceUpdate(
        type: WorkflowEventType,
        key: string,
        instance: JsWorkflowInstance,
        ts: string,
    ): Promise<void> {
        const pendingTasks =
            (await instance?.pending_tasks()) || new Int32Array([]);
        const visitedTasks =
            (await instance?.visited_tasks()) || new Int32Array([]);
        this.notify({
            type,
            payload: {
                key,
                ts: new Date(parseInt(ts)),
                pendingTasks,
                visitedTasks,
            },
        });
    }

    async cancel(isCompleted: boolean = false): Promise<void> {
        this.ignoreNextComplete = true;
        const c = this.context.routeContextProvider();
        if (c) {
            const { definitionId, instanceId } = c;
            if (definitionId && instanceId) {
                const instance = this.instances.getWith(c);
                if (instance) {
                    const remote_id = await instance.get_remote_id();
                    if (remote_id) {
                        const cancelled = !isCompleted;
                        await this.client?.complete(
                            CompleteRequest.create({
                                key: { id: remote_id.id() },
                                cancelled,
                            }),
                        );
                        await this.instances.deleteWith(c);
                        await this.variables.deleteWith(c);
                        await this.taskInfoLists.deleteWith(c);
                        if (isCompleted) {
                            this.notify({
                                type: WorkflowEventType.Completed,
                                payload: {
                                    completeTo: c.completeTo,
                                    definitionId,
                                    instanceId,
                                },
                            });
                        } else {
                            this.notify({
                                type: WorkflowEventType.Cancelled,
                                payload: {
                                    cancelTo: c.cancelTo,
                                    definitionId,
                                    instanceId,
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    async navigateTo(taskInfo: TaskInfo): Promise<void> {
        const { definitionId, instanceId } =
            this.context.routeContextProvider();
        if (definitionId && instanceId) {
            const instance = this.instances.getWith({
                definitionId,
                instanceId,
            });
            if (instance) {
                await this.navigateToTask(
                    definitionId,
                    instanceId,
                    instance,
                    taskInfo.n,
                );
            }
        }
    }

    async navigateToTask(
        definitionId: string,
        instanceId: string,
        instance: JsWorkflowInstance,
        task: number,
    ): Promise<void> {
        await instance.navigate_to(task);
        await this.updateRoute(definitionId, instanceId, task, false);
        await this.updateNav(definitionId, instanceId, instance);
    }

    async navigateToTaskId(
        definitionId: string,
        instanceId: string,
        taskId: string,
    ): Promise<void> {
        const idx = WorkflowDefinitions.taskIds[definitionId].findIndex(
            (t) => t === taskId,
        );
        const instance = this.instances.getWith({ definitionId, instanceId });
        if (idx !== -1 && instance) {
            await this.navigateToTask(definitionId, instanceId, instance, idx);
        }
    }

    async startWith(workflow: Workflow, replace: boolean): Promise<void> {
        const instance = await workflow.definition?.start(workflow.instanceId);
        const active = await instance?.get_active();
        if (typeof active === 'number' && instance) {
            this.instances.setWith(
                {
                    definitionId: workflow.definitionId,
                    instanceId: workflow.instanceId,
                },
                instance,
            );
            const state = await instance.state();
            const result = await this.client?.create(
                CreateRequest.create({
                    ctx: workflow.instanceId,
                    def: workflow.definitionId,
                    ver: workflow.version,
                    state,
                }),
            );
            if (result?.response.key?.id) {
                await instance?.set_remote_id(
                    result?.response.key?.id,
                    BigInt(result?.response.ts),
                );
                workflow.remoteId = result?.response.key?.id;
                await this.instanceUpdate(
                    WorkflowEventType.LoadedInstance,
                    workflow.remoteId,
                    instance,
                    result?.response.ts,
                );
            }
            await this.updateRoute(
                workflow.definitionId,
                workflow.instanceId,
                active,
                replace,
            );
            await this.updateNav(
                workflow.definitionId,
                workflow.instanceId,
                instance,
            );
        }
    }

    async start(): Promise<void> {
        const { definitionId, instanceId } =
            this.context.routeContextProvider();
        if (definitionId && instanceId) {
            const definition = await WorkflowDefinitions.load(definitionId);
            const version = definition?.version() || '';
            await this.startWith(
                { definitionId, instanceId, definition, version, exist: false },
                false,
            );
        }
    }

    async back(): Promise<void> {
        const { definitionId, instanceId } =
            this.context.routeContextProvider();
        if (definitionId && instanceId) {
            const instance = this.instances.getWith({
                definitionId,
                instanceId,
            });
            if (instance) {
                const active = await instance.back();
                if (active !== -1) {
                    await this.navigateToTask(
                        definitionId,
                        instanceId,
                        instance,
                        active,
                    );
                }
            }
        }
    }

    async getVariables<V>(initialVariables: V): Promise<V> {
        const instance = this.instances.get();
        if (instance) {
            const active = await instance.get_active();
            const variables = this.variables.get();
            if (variables && variables[active]) {
                return variables[active] as V;
            }
        }
        return initialVariables;
    }

    async setVariables(object: object): Promise<void> {
        const c = this.context.routeContextProvider();
        if (c) {
            const { definitionId, instanceId } = c;
            if (definitionId && instanceId) {
                const instance = this.instances.getWith(c);
                if (instance) {
                    const active = await instance.get_active();
                    const variables = this.variables.getWith(c) || {};
                    if (active !== undefined && variables) {
                        variables[active] = object;
                        await instance.set_variables(active, object);
                        this.variables.set(variables);
                        await this.updateNav(
                            definitionId,
                            instanceId,
                            instance,
                        );
                    }
                }
            }
        }
    }

    async updateRoute(
        definitionId: string,
        instanceId: string,
        active: number,
        replace: boolean = false,
    ): Promise<void> {
        this.notify({
            type: WorkflowEventType.RouteUpdate,
            payload: {
                to: {
                    definitionId,
                    instanceId,
                    taskId: WorkflowDefinitions.taskId(definitionId, active),
                    task: active,
                    replace,
                },
            },
        });
    }

    async updateNav(
        definitionId: string,
        instanceId: string,
        instance: JsWorkflowInstance,
    ): Promise<void> {
        const taskInfoList = await this.createTaskInfoList(
            definitionId,
            instance,
        );
        this.taskInfoLists.setWith({ definitionId, instanceId }, taskInfoList);
        this.notify({
            type: WorkflowEventType.NavUpdate,
            payload: { taskInfoList },
        });
    }

    async updateTaskInfoListWith(
        workflow: Workflow,
        instance: JsWorkflowInstance,
    ): Promise<TaskInfo[]> {
        const { definitionId, instanceId } = workflow;
        const taskInfoList = await this.createTaskInfoList(
            definitionId,
            instance,
        );
        this.taskInfoLists.setWith({ definitionId, instanceId }, taskInfoList);
        return taskInfoList;
    }

    async createTaskInfoList(
        definitionId: string,
        instance: JsWorkflowInstance,
    ): Promise<TaskInfo[]> {
        const result: TaskInfo[] = [];
        const futureTasks = await instance.maybe_visited_tasks();
        const pendingTasks = await instance.pending_tasks();
        const visitedTasks = await instance.visited_tasks();
        if (futureTasks && pendingTasks && visitedTasks) {
            for (let i = 0; i < futureTasks.length; i++) {
                const idx = futureTasks[i];
                const id = WorkflowDefinitions.taskIds[definitionId][idx];
                result.push({
                    n: idx,
                    id,
                    visited:
                        visitedTasks.findIndex((tid) => tid === idx) !== -1,
                    current:
                        pendingTasks.findIndex((tid) => tid === idx) !== -1,
                });
            }
        }
        return result;
    }
}
