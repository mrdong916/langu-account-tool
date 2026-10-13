import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('@/layouts/BasicLayout.vue'),
    name: 'workspace',
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
