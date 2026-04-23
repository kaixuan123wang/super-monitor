import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';
import { useUserStore } from '@/stores/user';

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/login/index.vue'),
    meta: { public: true },
  },
  {
    path: '/register',
    name: 'Register',
    component: () => import('@/views/register/index.vue'),
    meta: { public: true },
  },
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
        redirect: '/tracking/events',
        children: [
          {
            path: 'events',
            name: 'TrackEvents',
            component: () => import('@/views/tracking/events/index.vue'),
            meta: { title: '事件管理' },
          },
          {
            path: 'events/:eventName',
            name: 'TrackEventDetail',
            component: () => import('@/views/tracking/events/detail.vue'),
            meta: { title: '事件详情' },
          },
          {
            path: 'analysis',
            name: 'TrackAnalysis',
            component: () => import('@/views/tracking/analysis/index.vue'),
            meta: { title: '事件分析' },
          },
          {
            path: 'funnel',
            name: 'TrackFunnel',
            component: () => import('@/views/tracking/funnel/index.vue'),
            meta: { title: '漏斗分析' },
          },
          {
            path: 'retention',
            name: 'TrackRetention',
            component: () => import('@/views/tracking/retention/index.vue'),
            meta: { title: '留存分析' },
          },
          {
            path: 'users',
            name: 'TrackUsers',
            component: () => import('@/views/tracking/users/index.vue'),
            meta: { title: '用户画像' },
          },
          {
            path: 'debug',
            name: 'TrackDebug',
            component: () => import('@/views/tracking/debug/index.vue'),
            meta: { title: '实时事件流' },
          },
        ],
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

router.beforeEach((to, _from, next) => {
  const userStore = useUserStore();
  const isPublic = to.meta?.public === true;
  const isLoggedIn = userStore.isLoggedIn;

  if (!isPublic && !isLoggedIn) {
    next('/login');
  } else if ((to.path === '/login' || to.path === '/register') && isLoggedIn) {
    next('/');
  } else {
    next();
  }
});

export default router;
