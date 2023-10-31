import { type TaskInfo } from '.';

export interface DefinitionEvent {
    id: string;
    version: string;
}

export interface InstanceEvent {
    key: string;
    ts: Date;
    pendingTasks: Int32Array;
    visitedTasks: Int32Array;
}

export interface CompleteEvent {
    completeTo?: string;
    cancelTo?: string;
    instanceId: string;
    definitionId: string;
}

export interface NavUpdateEvent {
    taskInfoList: TaskInfo[];
}

export interface RouteTo {
    definitionId: string;
    instanceId: string;
    taskId?: string;
    task?: number;
    replace: boolean;
}

export interface RouteUpdateEvent {
    to: RouteTo;
}

export type WorkflowEventPayload =
    | DefinitionEvent
    | InstanceEvent
    | RouteUpdateEvent
    | NavUpdateEvent
    | CompleteEvent
    | null;

export enum WorkflowEventType {
    ResumeStart = 1,
    ResumeEnd = 2,
    ResumeError = 3,
    LoadDefinition = 4,
    LoadedDefinition = 5,
    LoadInstance = 6,
    LoadedInstance = 7,
    InstanceUpdate = 8,
    RouteUpdate = 9,
    NavUpdate = 10,
    Completed = 11,
    Cancelled = 12,
}

export interface WorkflowEvent {
    type: WorkflowEventType;
    payload: WorkflowEventPayload;
}
