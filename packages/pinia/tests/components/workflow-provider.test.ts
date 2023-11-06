import { shallowMount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';

import WorkflowProvider from '../../src/components/WorkflowProvider.vue';

beforeEach(() => {
    setActivePinia(createPinia());
});

test('WorkflowProvider', () => {
    const vueWrapper = shallowMount(WorkflowProvider);
    expect(vueWrapper.find('router-view').exists()).toBeTruthy();
});
