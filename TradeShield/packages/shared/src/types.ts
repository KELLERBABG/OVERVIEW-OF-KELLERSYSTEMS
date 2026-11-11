// Organization types
export type OrganizationType = 'laundry' | 'brand' | 'admin';

export interface Organization {
  id: string;
  name: string;
  type: OrganizationType;
  created_at: string;
}

// User types
export interface User {
  id: string;
  org_id: string;
  email: string;
  name: string | null;
  role: string;
  created_at: string;
}

// Device types
export type DeviceStatus = 'active' | 'inactive' | 'maintenance';

export interface Device {
  id: string;
  org_id: string;
  name: string | null;
  location: string | null;
  installed_at: string | null;
  status: DeviceStatus;
  created_at: string;
}

// Capture event types
export interface CaptureEvent {
  id: number;
  device_id: string;
  timestamp: string;
  water_volume_l: number;
  estimated_mg: number;
  wash_cycle: number;
  received_at: string;
}

// Capture aggregate types
export interface CaptureAggregate {
  id: number;
  device_id: string;
  period_start: string;
  period_end: string;
  total_cycles: number;
  total_water_l: number;
  total_mg: number;
  created_at: string;
}

// Credit batch types
export type CreditStatus = 'pending' | 'issued' | 'redeemed';

export interface CreditBatch {
  id: string;
  org_id: string;
  source_org_id: string;
  period_start: string;
  period_end: string;
  total_mg: number;
  total_eur: number;
  status: CreditStatus;
  issued_at: string | null;
  created_at: string;
}

// API request/response types
export interface CreateEventRequest {
  device_id: string;
  timestamp: string;
  water_volume_l: number;
  estimated_mg: number;
  wash_cycle: number;
}

export interface CreateEventResponse {
  id: number;
  device_id: string;
  wash_cycle: number;
  received_at: string;
}

export interface CreateBatchEventsResponse {
  accepted: number;
  rejected: number;
  events: CreateEventResponse[];
  errors: Array<{ wash_cycle: number; error: string }>;
}

export interface DeviceHeartbeatRequest {
  device_id: string;
  timestamp: string;
  firmware_version?: string;
  battery_level?: number;
}

export interface DeviceHeartbeatResponse {
  status: 'ok';
  server_time: string;
  next_check_in_ms: number;
}

export interface DeviceWithStats extends Device {
  total_cycles: number;
  total_mg: number;
  total_water_l: number;
  recent_events?: CaptureEvent[];
}

export interface DashboardSummary {
  total_devices: number;
  active_devices: number;
  total_events_30d: number;
  total_mg_30d: number;
  total_water_l_30d: number;
  total_credits_mg: number;
  total_credits_eur: number;
  comparison?: {
    total_events_prev: number;
    total_mg_prev: number;
    events_change_pct: number;
    mg_change_pct: number;
  };
}

export interface TimelinePoint {
  date: string;
  total_mg: number;
  total_water_l: number;
  event_count: number;
}

export interface DeviceRanking {
  device_id: string;
  device_name: string | null;
  total_mg: number;
  total_cycles: number;
  rank: number;
}

export interface IssueCreditRequest {
  source_org_id: string;
  period_start: string;
  period_end: string;
}

export interface IssueCreditResponse {
  id: string;
  total_mg: number;
  total_eur: number;
  status: 'pending';
  created_at: string;
}

export interface CreditAuditEntry {
  id: number;
  credit_id: string;
  action: string;
  from_status: string;
  to_status: string;
  performed_by: string | null;
  notes: string | null;
  created_at: string;
}
