import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import LoginView from '../views/LoginView.vue'
import DashboardLayout from '../views/DashboardLayout.vue'
import OverviewView from '../views/OverviewView.vue'
import DriversView from '../views/DriversView.vue'
import PitsView from '../views/PitsView.vue'
import WaybillsView from '../views/WaybillsView.vue'
import QueueView from '../views/QueueView.vue'
import AlertsView from '../views/AlertsView.vue'

const routes = [
  { path: '/login', name: 'login', component: LoginView, meta: { public: true } },
  {
    path: '/',
    component: DashboardLayout,
    children: [
      { path: '', name: 'overview', component: OverviewView },
      { path: 'drivers', name: 'drivers', component: DriversView },
      { path: 'pits', name: 'pits', component: PitsView },
      { path: 'waybills', name: 'waybills', component: WaybillsView },
      { path: 'queue', name: 'queue', component: QueueView },
      { path: 'alerts', name: 'alerts', component: AlertsView },
    ],
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to, _from, next) => {
  const auth = useAuthStore()
  if (!to.meta.public && !auth.isLoggedIn) {
    next('/login')
  } else {
    next()
  }
})

export default router
