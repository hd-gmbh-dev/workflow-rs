import { describe, expect, it } from '@jest/globals';

import { type WorkflowContextProvider, WorkflowRs } from '../src/workflows';

let workflowRs: WorkflowRs;

beforeEach(() => {
    workflowRs = new WorkflowRs();
});

describe('workflow-rs class', () => {
    it('tests setContext function', () => {
        const c: WorkflowContextProvider = {
            routeContextProvider: jest.fn(),
            authContextProvider: jest.fn(),
        };

        workflowRs.setContext(c);

        expect(workflowRs.context).toStrictEqual(c);
    });
});
