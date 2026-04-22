import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('@/components/layout/BasicLayout.vue'),
    redirect: '/dashboard',
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/dashboard/index.vue'),
        meta: { title: '概览', icon: 'DataLine' },
      },
      {
        path: 'projects',
        name: 'Projects',
        component: () => import('@/views/project/index.vue'),
        meta: { title: '项目管理', icon: 'Folder' },
      },
      {
        path: 'errors',
        name: 'Errors',
        component: () => import('@/views/errors/index.vue'),
        meta: { title: '错误监控', icon: 'Warning' },
      },
      {
        path: 'network',
        name: 'Network',
        component: () => import('@/views/network/index.vue'),
        meta: { title: '接口监控', icon: 'Connection' },
      },
      {
        path: 'sourcemap',
        name: 'Sourcemap',
        component: () => import('@/views/sourcemap/index.vue'),
        meta: { title: 'Source Map', icon: 'Document' },
      },
      {
        path: 'ai-analysis',
        name: 'AIAnalysis',
        component: () => import('@/views/ai-analysis/index.vue'),
        meta: { title: 'AI 分析', icon: 'MagicStick' },
      },
      {
        path: 'tracking',
        name: 'Tracking',
        component: () => import('@/views/tracking/index.vue'),
        meta: { title: '用户埋点', icon: 'Aim' },
      },
      {
        path: 'users',
        name: 'Users',
        component: () => import('@/views/user/index.vue'),
        meta: { title: '用户管理', icon: 'User' },
      },
      {
        path: 'groups',
        name: 'Groups',
        component: () => import('@/views/group/index.vue'),
        meta: { title: '分组管理', icon: 'UserFilled' },
      },
      {
        path: 'settings',
        name: 'Settings',
        component: () => import('@/views/settings/index.vue'),
        meta: { title: '告警设置', icon: 'Bell' },
      },
    ],
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
