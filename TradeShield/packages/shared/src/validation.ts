import { z } from 'zod';

// ─── Event schemas ───────────────────────────────────────────────
const DeviceIdField = z.string().min(1).max(64);
const TimestampField = z.string().datetime();
const DateField = z.string().regex(/^\d{4}-\d{2}-\d{2}$/);

export const CreateEventSchema = z.object({
  device_id: DeviceIdField,
  timestamp: TimestampField,
  water_volume_l: z.number().positive().max(100),
  estimated_mg: z.number().nonnegative().max(100),
  wash_cycle: z.number().int().positive(),
});

export const CreateBatchEventsSchema = z.object({
  events: z.array(CreateEventSchema).min(1).max(50),
});

// ─── Device schemas ──────────────────────────────────────────────
export const DeviceHeartbeatSchema = z.object({
  device_id: DeviceIdField,
  timestamp: TimestampField,
  firmware_version: z.string().max(32).optional(),
  battery_level: z.number().min(0).max(100).optional(),
});

export const RegisterDeviceSchema = z.object({
  id: DeviceIdField,
  name: z.string().min(1).max(100).optional(),
  location: z.string().max(200).optional(),
  installed_at: TimestampField.optional(),
});

export const UpdateDeviceSchema = z.object({
  name: z.string().min(1).max(100).optional(),
  location: z.string().max(200).optional(),
  status: z.enum(['active', 'inactive', 'maintenance']).optional(),
});

export const DeviceStatusSchema = z.object({
  status: z.enum(['active', 'inactive', 'maintenance']),
});

// ─── Credit schemas ──────────────────────────────────────────────
export const IssueCreditSchema = z.object({
  org_id: z.string().min(1).optional(),  // receiver (brand). Defaults to authenticated user's org.
  source_org_id: z.string().min(1),
  period_start: DateField,
  period_end: DateField,
});

export const RedeemCreditSchema = z.object({
  notes: z.string().max(500).optional(),
});

// ─── Auth schemas ────────────────────────────────────────────────
export const LoginSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8).max(128),
});

// ─── Query parameter schemas ─────────────────────────────────────
export const PaginationSchema = z.object({
  page: z.coerce.number().int().positive().default(1),
  limit: z.coerce.number().int().positive().max(200).default(50),
});

export const EventFilterSchema = PaginationSchema.extend({
  device_id: DeviceIdField.optional(),
  start_date: DateField.optional(),
  end_date: DateField.optional(),
});

export const TimelineSchema = z.object({
  period: z.enum(['day', 'week', 'month']).default('day'),
  start_date: DateField.optional(),
  end_date: DateField.optional(),
});

export const CreditFilterSchema = PaginationSchema.extend({
  status: z.enum(['pending', 'issued', 'redeemed']).optional(),
  source_org_id: z.string().optional(),
});

export const DashboardSummarySchema = z.object({
  compare: z.coerce.boolean().default(false),
});

// ─── Types derived from schemas ──────────────────────────────────
export type CreateEventInput = z.infer<typeof CreateEventSchema>;
export type CreateBatchEventsInput = z.infer<typeof CreateBatchEventsSchema>;
export type DeviceHeartbeatInput = z.infer<typeof DeviceHeartbeatSchema>;
export type IssueCreditInput = z.infer<typeof IssueCreditSchema>;
export type RedeemCreditInput = z.infer<typeof RedeemCreditSchema>;
export type RegisterDeviceInput = z.infer<typeof RegisterDeviceSchema>;
export type UpdateDeviceInput = z.infer<typeof UpdateDeviceSchema>;
export type LoginInput = z.infer<typeof LoginSchema>;
export type PaginationInput = z.infer<typeof PaginationSchema>;
export type EventFilterInput = z.infer<typeof EventFilterSchema>;
export type TimelineInput = z.infer<typeof TimelineSchema>;
export type CreditFilterInput = z.infer<typeof CreditFilterSchema>;
