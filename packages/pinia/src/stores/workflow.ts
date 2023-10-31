import { defineStore } from 'pinia';
import { ref, type Ref } from 'vue';
import {
    useRoute,
    useRouter,
    type Router,
    type RouteLocationNormalized,
    type RouteRecordRaw,
} from 'vue-router';
import {
    type TaskInfo,
    WorkflowRs,
    WorkflowSource,
    type RouteContextProvider,
    type RouteCtx,
    type AuthContextProvider,
    type AuthCtx,
    WorkflowEventType,
    type DefinitionEvent,
    type RouteUpdateEvent,
    type NavUpdateEvent,
    type InstanceEvent,
    type CompleteEvent,
} from '@wfrs/core';
import { type SubscriptionLike } from 'rxjs';
export { type TaskInfo, WorkflowSource };

function routeContextProvider(): RouteContextProvider {
    const route = useRoute();
    return (): RouteCtx => {
        return {
            definitionId: (route?.meta?.workflow as WorkflowSource)?.key,
            instanceId: route?.params?.instanceId as string,
            completeTo: route?.meta?.completeTo as string,
            cancelTo: route?.meta?.cancelTo as string,
        };
    };
}

function definitonFromRoutes(
    state: Map<string, WorkflowSource>,
    route: RouteRecordRaw,
): Map<string, WorkflowSource> {
    const workflow = route.meta?.workflow ?? null;
    if (workflow instanceof WorkflowSource) {
        state.set(workflow.key, workflow);
    }
    const children = route.children ?? null;
    if (children !== null) {
        children.reduce(
            (state, current) => definitonFromRoutes(state, current),
            state,
        );
    }
    return state;
}

export const useWorkflowStore = defineStore('wfrs-workflow', () => {
    const taskInfoList = ref<TaskInfo[]>([]);
    // not implemented yet
    // const active: Ref<any[]> = ref([]);
    const isLoading: Ref<boolean> = ref(false);
    const definitions: Ref<string[]> = ref([]);
    const workflowRs = new WorkflowRs();
    const activeDefinitionId = ref<string | null>(null);
    const activeTask = ref<number>(-1);
    const activeTaskId = ref<string>('');
    const activeDefinitionVersion = ref<string | null>(null);
    const activeRemoteKey = ref<string | null>(null);
    const activeRemoteVersion = ref<Date | null>(null);
    const activeUserId = ref<string | null>(null);
    const activeAuthToken = ref<string | null>(null);
    const activePendingTasks = ref<number[]>([]);
    const activeVisitedTasks = ref<number[]>([]);

    function handleRouteEvent(
        router: Router,
        route: RouteLocationNormalized,
        event: RouteUpdateEvent,
    ): void {
        const workflow: WorkflowSource | null =
            (route.meta?.workflow as WorkflowSource) ?? null;
        const instanceId = route.params?.instanceId ?? null;
        if (
            workflow.key === event.to.definitionId &&
            instanceId === event.to.instanceId
        ) {
            const baseUrl = `${event.to.definitionId}/${event.to.instanceId}`;
            const i = route.path.indexOf(baseUrl);
            const base = route.path.slice(0, i - 1);
            const instanceUrl = `${base}/${event.to.definitionId}/${event.to.instanceId}/${event.to.taskId}`;
            if (!route.path.startsWith(instanceUrl)) {
                if (event.to.replace) {
                    void router.replace(instanceUrl);
                } else {
                    void router.push(instanceUrl);
                }
            }
        }
        const taskId = event.to.taskId ?? null;
        if (taskId !== null) {
            activeTaskId.value = taskId;
        }
        const task = event.to.task ?? null;
        if (task !== null) {
            activeTask.value = task;
        }
    }

    function handleCompleteEvent(
        router: Router,
        route: RouteLocationNormalized,
        event: CompleteEvent | null,
    ): void {
        const completeTo = event?.completeTo ?? '';
        const definitionId = event?.definitionId ?? null;
        const instanceId = event?.instanceId ?? null;
        if (definitionId !== null && instanceId !== null && completeTo !== '') {
            if (completeTo.endsWith(`${definitionId}/:instanceId`)) {
                const i = route.path.indexOf(definitionId);
                const base = route.path.slice(0, i - 1);
                const targetUrl = `${base}/${definitionId}/${instanceId}`;
                void router.push(targetUrl);
            } else {
                void router.push(completeTo);
            }
            return;
        }
        void router.push('/');
    }

    function handleCancelEvent(
        router: Router,
        route: RouteLocationNormalized,
        event: CompleteEvent | null,
    ): void {
        const cancelTo = event?.cancelTo ?? '';
        const definitionId = event?.definitionId ?? null;
        const instanceId = event?.instanceId ?? null;
        if (definitionId !== null && instanceId !== null && cancelTo !== '') {
            if (cancelTo.endsWith(`${definitionId}/:instanceId`)) {
                const i = route.path.indexOf(definitionId);
                const base = route.path.slice(0, i - 1);
                const targetUrl = `${base}/${definitionId}/${instanceId}`;
                void router.push(targetUrl);
            } else {
                void router.push(cancelTo);
            }
            return;
        }
        void router.push('/');
    }

    function authContextProvider(): AuthContextProvider {
        return (): AuthCtx => {
            return {
                userId: activeUserId.value ?? undefined,
                token: activeAuthToken.value ?? undefined,
            };
        };
    }

    let subscription: SubscriptionLike | null = null;

    async function registerDefinitons(routes: RouteRecordRaw[]): Promise<void> {
        const result = routes.reduce(definitonFromRoutes, new Map());
        definitions.value = workflowRs.register(result.values());
    }

    function onLogin(userId: string, token: string): void {
        activeUserId.value = userId;
        activeAuthToken.value = token;
    }

    function onRouteLeave(
        _to: RouteLocationNormalized,
        _from: RouteLocationNormalized,
    ): void {
        // TODO: maybe remove if not needed at all
    }

    async function onRouteUpdate(to: RouteLocationNormalized): Promise<void> {
        const definitionId = (to.meta?.workflow as WorkflowSource)?.key ?? null;
        const instanceId = (to.params?.instanceId as string) ?? null;
        const taskId = activeTaskId.value ?? '';
        if (definitionId !== null && instanceId !== null && taskId !== '') {
            const i = to.path.indexOf(instanceId);
            const base = to.path.slice(0, i - 1);
            const instanceUrl = `${base}/${instanceId}`;
            const targetUrl = `${instanceUrl}/${taskId}`;
            if (targetUrl !== to.path && to.path.startsWith(instanceUrl)) {
                const nextTaskId = to.path.slice(instanceUrl.length + 1);
                await workflowRs.navigateToTaskId(
                    definitionId,
                    instanceId,
                    nextTaskId,
                );
            }
        }
    }

    async function resume(): Promise<void> {
        const router = useRouter();
        const route = useRoute();
        isLoading.value = true;
        subscription = workflowRs.events.subscribe((event) => {
            switch (event.type) {
                case WorkflowEventType.LoadDefinition:
                    // TODO: maybe set some loading state for the definition
                    break;
                case WorkflowEventType.LoadedDefinition:
                    activeDefinitionId.value = (
                        event.payload as DefinitionEvent
                    ).id;
                    activeDefinitionVersion.value =
                        (event.payload as DefinitionEvent).version ?? '';
                    break;
                case WorkflowEventType.LoadInstance:
                    // TODO: maybe set some loading state for the instance
                    break;
                case WorkflowEventType.LoadedInstance:
                case WorkflowEventType.InstanceUpdate:
                    activeRemoteKey.value = (
                        event.payload as InstanceEvent
                    ).key;
                    activeRemoteVersion.value = (
                        event.payload as InstanceEvent
                    ).ts;
                    activePendingTasks.value = [
                        ...Array.from(
                            (event.payload as InstanceEvent).pendingTasks,
                        ),
                    ];
                    activeVisitedTasks.value = [
                        ...Array.from(
                            (event.payload as InstanceEvent).visitedTasks,
                        ),
                    ];
                    break;
                case WorkflowEventType.RouteUpdate:
                    handleRouteEvent(
                        router,
                        route,
                        event.payload as RouteUpdateEvent,
                    );
                    break;
                case WorkflowEventType.NavUpdate:
                    taskInfoList.value = (
                        event.payload as NavUpdateEvent
                    ).taskInfoList;
                    break;
                case WorkflowEventType.Completed:
                    handleCompleteEvent(
                        router,
                        route,
                        event.payload as CompleteEvent,
                    );
                    break;
                case WorkflowEventType.Cancelled:
                    handleCancelEvent(
                        router,
                        route,
                        event.payload as CompleteEvent,
                    );
                    break;
                default:
                    // TODO: maybe set more states for other events
                    break;
            }
        });
        workflowRs.setContext({
            routeContextProvider: routeContextProvider(),
            authContextProvider: authContextProvider(),
        });
        await workflowRs.resume();
        isLoading.value = false;
    }

    function leave(): void {
        if (subscription !== null) {
            subscription.unsubscribe();
            subscription = null;
        }
        activeDefinitionId.value = null;
        activeTask.value = -1;
        activeTaskId.value = '';
        activeDefinitionVersion.value = null;
        activeRemoteKey.value = null;
        activeRemoteVersion.value = null;
        activePendingTasks.value = [];
        activeVisitedTasks.value = [];
        workflowRs.leave();
    }

    const complete = workflowRs.complete.bind(workflowRs);
    const navigateTo = workflowRs.navigateTo.bind(workflowRs);
    const start = workflowRs.start.bind(workflowRs);
    const back = workflowRs.back.bind(workflowRs);
    const cancel = async (): Promise<void> => {
        await workflowRs.cancel(false);
    };
    const getVariables = workflowRs.getVariables.bind(workflowRs);
    const setVariables = workflowRs.setVariables.bind(workflowRs);

    return {
        isLoading,
        activeDefinitionId,
        activeDefinitionVersion,
        activeRemoteKey,
        activeRemoteVersion,
        activeTask,
        activePendingTasks,
        activeVisitedTasks,
        registerDefinitons,
        onLogin,
        onRouteLeave,
        onRouteUpdate,
        resume,
        leave,
        navigateTo,
        taskInfoList,
        definitions,
        start,
        back,
        cancel,
        complete,
        getVariables,
        setVariables,
    };
});
