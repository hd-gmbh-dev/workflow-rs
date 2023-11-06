import { shallowMount } from '@vue/test-utils';

import WorkflowSideBar from '../../src/components/WorkflowSideBar.vue';

test('WorkflowSideBar', () => {
    const vueWrapper = shallowMount(WorkflowSideBar, {
        propsData: {
            items: [],
        },
    });
    expect(vueWrapper.find('ul').exists()).toBeTruthy();
});
