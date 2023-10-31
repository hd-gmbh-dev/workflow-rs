<script lang="ts" setup>
import { useWorkflowStore } from '@/stores/workflow';

const workflowStore = useWorkflowStore();
const { back, cancel, complete } = workflowStore;
</script>
<template>
    <slot :back="back" :cancel="cancel" :complete="complete">
        <div>
            <button data-test-id="@wfrs/pinia-btn-back" @click="back">
                back
            </button>
            <button
                data-test-id="@wfrs/pinia-btn-cancel"
                class="pl-4"
                @click="cancel"
            >
                cancel
            </button>
            <button
                data-test-id="@wfrs/pinia-btn-complete"
                class="pl-4"
                @click="complete"
            >
                complete
            </button>
        </div>
    </slot>
</template>
