<script lang="ts" setup>
import { type TaskInfo } from '@/stores/workflow';
const emit = defineEmits(['navigateTo']);
defineProps<{ items: TaskInfo[] }>();
</script>
<template>
    <ul class="nav">
        <li
            :class="[
                'nav-item',
                {
                    active: taskInfo.current,
                    visited: !taskInfo.current && taskInfo.visited,
                },
            ]"
            :key="taskInfo.id"
            v-for="taskInfo in items"
            @click="emit('navigateTo', taskInfo)"
        >
            {{ taskInfo.id }}
        </li>
    </ul>
</template>
