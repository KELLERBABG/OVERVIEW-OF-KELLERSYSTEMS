// Microplastic estimation constants
// Based on Plymouth University research and industry standards
export const MICROPLASTIC_PER_LITER_MG = 0.12; // Average mixed synthetic load
export const CONSERVATIVE_MICROPLASTIC_PER_LITER_MG = 0.08; // For credit calculations (credibility > optimism)

// Credit pricing (EUR per mg)
export const CREDIT_PRICE_PER_MG_EUR = 0.001; // 1 EUR per gram

// Device thresholds
export const DEVICE_HEARTBEAT_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes
export const DEVICE_TIMEOUT_MS = 15 * 60 * 1000; // 15 minutes (3 missed heartbeats)

// API
export const API_KEY_HEADER = 'X-API-Key';
export const SESSION_COOKIE = 'session';

// Pagination
export const DEFAULT_PAGE_SIZE = 50;
export const MAX_PAGE_SIZE = 200;

// Date formats
export const DATE_FORMAT = 'YYYY-MM-DD';
export const DATETIME_FORMAT = 'YYYY-MM-DD HH:mm:ss';
