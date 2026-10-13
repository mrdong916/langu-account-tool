<script setup lang="ts">
import { ref, watch } from 'vue';
import { UserRound } from 'lucide-vue-next';
const props = defineProps<{ username?: string; src?: string; primary?: boolean }>();
const failed = ref(false);
watch(
  () => props.src,
  () => {
    failed.value = false;
  },
);
</script>

<template>
  <span class="avatar" :class="{ 'avatar-primary': primary }">
    <img
      v-if="src && !failed"
      :src="src"
      alt=""
      referrerpolicy="no-referrer"
      @error="failed = true"
    />
    <span v-else-if="username">{{ username.slice(0, 1).toUpperCase() }}</span>
    <UserRound v-else :size="18" />
  </span>
</template>
