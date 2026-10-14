<script setup lang="ts">
import { computed } from 'vue';
const props = defineProps<{ text: string; query: string }>();
const parts = computed(() => {
  const tokens = props.query
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  if (!tokens.length) return [{ text: props.text, hit: false }];
  const pattern = new RegExp(`(${tokens.join('|')})`, 'ig');
  return props.text.split(pattern).map((text, index) => ({ text, hit: index % 2 === 1 }));
});
</script>
<template>
  <template v-for="(part, index) in parts" :key="index"
    ><mark v-if="part.hit">{{ part.text }}</mark
    ><template v-else>{{ part.text }}</template></template
  >
</template>
