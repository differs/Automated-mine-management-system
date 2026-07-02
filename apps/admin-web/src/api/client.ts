const BASE_URL = '/api/v1'

let _token: string | null = null

export function setToken(token: string | null) {
  _token = token
}

export function getToken(): string | null {
  return _token
}

// ── 分页响应类型 ──────────────────────────────────────────────
export interface PagedResponse<T> {
  data: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export interface PaginationParams {
  page?: number
  page_size?: number
}

// ── 通用请求 ──────────────────────────────────────────────────
async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  if (_token) {
    headers['Authorization'] = `Bearer ${_token}`
  }

  const res = await fetch(`${BASE_URL}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  })

  if (res.status === 401) {
    _token = null
    window.location.href = '/login'
    throw new Error('unauthorized')
  }

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }))
    throw new Error(err.message || 'request failed')
  }

  return res.json()
}

// ── Auth ──────────────────────────────────────────────────────
export const authApi = {
  login: (username: string, password: string) =>
    request<{ access_token: string; refresh_token: string; role: string; display_name: string }>(
      'POST', '/auth/login', { username, password },
    ),
  refresh: (refresh_token: string) =>
    request<{ access_token: string }>('POST', '/auth/refresh', { refresh_token }),
}

// ── Drivers ───────────────────────────────────────────────────
export const driverApi = {
  list: (params?: { keyword?: string; status?: string } & PaginationParams) => {
    const q = new URLSearchParams()
    if (params?.keyword) q.set('keyword', params.keyword)
    if (params?.status) q.set('status', params.status)
    if (params?.page) q.set('page', String(params.page))
    if (params?.page_size) q.set('page_size', String(params.page_size))
    const query = q.toString()
    return request<PagedResponse<{
      id: string; name: string; phone: string; license_plate: string; vehicle_type: string; status: string
    }>>('GET', `/drivers${query ? `?${query}` : ''}`)
  },
  get: (id: string) =>
    request<{
      id: string; name: string; phone: string; license_plate: string;
      vehicle_type: string; capacity_ton: number; status: string; updated_at: string
    }>('GET', `/drivers/${id}`),
  create: (data: { name: string; phone: string; license_plate: string; vehicle_type: string; capacity_ton: number }) =>
    request('POST', '/drivers', data),
  import_: (source: string, drivers: Array<{ name: string; phone: string; license_plate: string; vehicle_type: string; capacity_ton: number }>) =>
    request<{ accepted: boolean; source: string; imported: number; failed: number; errors: string[] }>(
      'POST', '/drivers/import', { source, drivers },
    ),
}

// ── Pits ──────────────────────────────────────────────────────
export const pitApi = {
  list: (params?: PaginationParams) => {
    const q = new URLSearchParams()
    if (params?.page) q.set('page', String(params.page))
    if (params?.page_size) q.set('page_size', String(params.page_size))
    const query = q.toString()
    return request<PagedResponse<{
      id: string; name: string; code: string; current_queue_count: number;
      avg_wait_minutes: number; is_active: boolean
    }>>('GET', `/pits${query ? `?${query}` : ''}`)
  },
  get: (id: string) =>
    request<{
      id: string; name: string; code: string; location_text: string | null;
      queue_capacity: number | null; current_queue_count: number;
      avg_wait_minutes: number; is_active: boolean
    }>('GET', `/pits/${id}`),
  create: (data: { name: string; code?: string; location_text?: string; queue_capacity?: number }) =>
    request('POST', '/pits', data),
}

// ── Waybills ──────────────────────────────────────────────────
export const waybillApi = {
  list: (params?: { status?: string; pit_id?: string } & PaginationParams) => {
    const q = new URLSearchParams()
    if (params?.status) q.set('status', params.status)
    if (params?.pit_id) q.set('pit_id', params.pit_id)
    if (params?.page) q.set('page', String(params.page))
    if (params?.page_size) q.set('page_size', String(params.page_size))
    const query = q.toString()
    return request<PagedResponse<{
      id: string; serial_no: string; driver_id: string; pit_id: string;
      status: string; dispatch_time: string | null
    }>>('GET', `/waybills${query ? `?${query}` : ''}`)
  },
  get: (id: string) =>
    request<{
      id: string; serial_no: string; driver_id: string; pit_id: string;
      status: string; queue_number: number | null; estimated_weight_ton: number | null;
      actual_weight_ton: number | null; dispatch_time: string | null; arrive_time: string | null
    }>('GET', `/waybills/${id}`),
  create: (data: { driver_id: string; pit_id: string; estimated_weight_ton?: number }) =>
    request('POST', '/waybills', data),
  dispatch: (id: string, dispatcher_id: string) =>
    request<{ id: string; status: string; at: string }>('POST', `/waybills/${id}/dispatch`, { dispatcher_id }),
  arrive: (id: string, arrival_source: string) =>
    request<{ id: string; status: string; at: string }>('POST', `/waybills/${id}/arrive`, { arrival_source }),
  cancel: (id: string, cancelled_by: string, reason: string) =>
    request<{ id: string; status: string; at: string }>('POST', `/waybills/${id}/cancel`, { cancelled_by, reason }),
}

// ── Queue ─────────────────────────────────────────────────────
export const queueApi = {
  getPitQueue: (pitId: string) =>
    request<Array<{
      waybill_id: string; driver_id: string; queue_position: number; entered_at: string
    }>>('GET', `/queue/pits/${pitId}`),
  join: (waybillId: string, driver_id: string, pit_id: string, arrival_method: string) =>
    request('POST', `/queue/waybills/${waybillId}/join`, { driver_id, pit_id, arrival_method }),
  callNext: (waybillId: string, operator_id: string, reason?: string) =>
    request('POST', `/queue/waybills/${waybillId}/call-next`, { operator_id, reason }),
  leave: (waybillId: string, operator_id: string, reason: string) =>
    request('POST', `/queue/waybills/${waybillId}/leave`, { operator_id, reason }),
}

// ── Dashboard ─────────────────────────────────────────────────
export const dashboardApi = {
  overview: () =>
    request<{
      today_total_waybills: number
      today_completed: number
      today_cancelled: number
      in_progress: number
      today_total_tonnage: number
      pit_summaries: Array<{
        pit_id: string; pit_name: string; current_queue: number;
        avg_wait_minutes: number; today_trips: number; today_tonnage: number
      }>
      date: string
    }>('GET', '/dashboard/overview'),
  pitEfficiency: () =>
    request<Array<{
      pit_id: string; pit_name: string; today_trips: number;
      today_tonnage: number; avg_wait_minutes: number; avg_loading_minutes: number
    }>>('GET', '/dashboard/pit-efficiency'),
  driverRanking: () =>
    request<Array<{
      driver_id: string; driver_name: string; license_plate: string;
      today_trips: number; today_tonnage: number
    }>>('GET', '/dashboard/driver-ranking'),
}

// ── Alerts ────────────────────────────────────────────────────
export const alertApi = {
  list: (params?: { status?: string; type?: string } & PaginationParams) => {
    const q = new URLSearchParams()
    if (params?.status) q.set('status', params.status)
    if (params?.type) q.set('type', params.type)
    if (params?.page) q.set('page', String(params.page))
    if (params?.page_size) q.set('page_size', String(params.page_size))
    const query = q.toString()
    return request<PagedResponse<{
      id: string; waybill_id: string; type: string; severity: number;
      description: string; status: string; created_at: string; resolved_at: string | null
    }>>('GET', `/alerts${query ? `?${query}` : ''}`)
  },
  resolve: (id: string) =>
    request<{ id: string; status: string; resolved_at: string }>('POST', `/alerts/${id}/resolve`),
}

// ── Dispatch ──────────────────────────────────────────────────
export const dispatchApi = {
  recommendations: (top_n?: number) => {
    const q = new URLSearchParams()
    if (top_n) q.set('top_n', String(top_n))
    const query = q.toString()
    return request<{
      plan: {
        recommendations: Array<{
          rank: number; waybill_id: string; driver_id: string; driver_name: string;
          pit_id: string; pit_name: string; composite_score: number;
          congestion_level: string; reason: string
        }>
        total_pending: number
        total_idle_drivers: number
        algorithm_version: string
      }
      cached: boolean
    }>('GET', `/dispatch/recommendations${query ? `?${query}` : ''}`)
  },
}
