<script lang="ts" setup>
import { onBeforeUnmount, onMounted } from 'vue';
import { useWorkflowStore } from '@/stores/workflow';
import { onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router';

const workflowStore = useWorkflowStore();
onBeforeRouteLeave(workflowStore.onRouteLeave);
onBeforeRouteUpdate(workflowStore.onRouteUpdate);
onMounted(workflowStore.resume);
onBeforeUnmount(workflowStore.leave);
</script>
<template>
    <div>
        <template v-if="!workflowStore.isLoading">
            <slot
                name="sidebar"
                :navigateTo="workflowStore.navigateTo"
                :items="workflowStore.taskInfoList"
            ></slot>
            <slot name="content">
                <router-view />
            </slot>
        </template>
        <slot v-else name="loading"></slot>
    </div>
</template>
