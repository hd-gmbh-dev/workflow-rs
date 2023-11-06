import { createPinia, setActivePinia } from 'pinia';
import { useWorkflowStore } from '../../src/stores/workflow';

beforeEach(() => {
    setActivePinia(createPinia());
});

describe('workflow', () => {
    it('tests useWorkflowStore function', () => {
        const s = useWorkflowStore();
        expect(s.isLoading).toBeFalsy();
    });
});
