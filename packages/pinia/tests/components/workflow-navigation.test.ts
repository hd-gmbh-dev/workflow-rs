import { shallowMount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';

import WorkflowNavigation from '../../src/components/WorkflowNavigation.vue';

beforeEach(() => {
    setActivePinia(createPinia());
});

test('WorkflowNavigation', () => {
    const vueWrapper = shallowMount(WorkflowNavigation);
    expect(vueWrapper.find('div').exists()).toBeTruthy();
});
