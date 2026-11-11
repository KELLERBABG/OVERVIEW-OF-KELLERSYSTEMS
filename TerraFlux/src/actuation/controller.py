"""Actuation layer: turn an advisory into an executable valve plan.

The engine only *recommends*; this package is the boundary where a recommendation can
be handed to real hardware. Two controllers are available:

* ``SimulatedValveController`` - the default. It records the command and reports it as
  simulated, so the whole flow can be exercised without a valve in the field.
* ``HttpValveController`` - posts the plan to a valve gateway (for example an ESP32
  relay controller) via ``VALVE_CONTROLLER_URL``.

Nothing is executed automatically unless ``AUTO_ACTUATION_ENABLED`` is switched on.
"""
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Dict, Optional

from ..core.config import settings
from ..core.logger import logger


@dataclass
class ValvePlan:
    """An executable irrigation instruction for one valve."""
    field_id: int
    field_name: str
    start_time: str                      # 'HH:MM' in the farm's local time
    duration_minutes: int
    gross_depth_mm: float                # what the valve has to deliver
    net_depth_mm: float                  # what reaches the root zone
    application_rate_mm_h: float
    irrigation_efficiency: float
    reason: str = ''
    zone: Optional[str] = None
    valve_id: Optional[str] = None
    created_at: datetime = field(default_factory=lambda: datetime.now(timezone.utc))

    def to_dict(self) -> Dict[str, Any]:
        return {
            'field_id': self.field_id,
            'field_name': self.field_name,
            'zone': self.zone,
            'valve_id': self.valve_id,
            'start_time': self.start_time,
            'duration_minutes': self.duration_minutes,
            'gross_depth_mm': self.gross_depth_mm,
            'net_depth_mm': self.net_depth_mm,
            'application_rate_mm_h': self.application_rate_mm_h,
            'irrigation_efficiency': self.irrigation_efficiency,
            'reason': self.reason,
            'created_at': self.created_at.isoformat()
        }


class ValveController:
    """Interface every valve backend has to implement."""

    def open_valve(self, plan: ValvePlan) -> Dict[str, Any]:
        raise NotImplementedError


class SimulatedValveController(ValveController):
    """Records the command without touching any hardware."""

    def open_valve(self, plan: ValvePlan) -> Dict[str, Any]:
        logger.info(
            f"[ACTUATION SIMULATED] {plan.valve_id or 'default valve'} | zone={plan.zone or '-'} | "
            f"{plan.gross_depth_mm:.1f} mm gross for {plan.duration_minutes} min at {plan.start_time}"
        )
        return {
            'status': 'simulated',
            'valve_id': plan.valve_id,
            'zone': plan.zone,
            'duration_minutes': plan.duration_minutes,
            'start_time': plan.start_time,
            'mock': True
        }


class HttpValveController(ValveController):
    """Posts the plan to a valve gateway over HTTP."""

    def __init__(self, base_url: Optional[str] = None, token: Optional[str] = None, timeout: float = 8.0):
        self.base_url = (base_url or settings.VALVE_CONTROLLER_URL or '').rstrip('/')
        self.token = token or settings.VALVE_CONTROLLER_TOKEN
        self.timeout = timeout

    def open_valve(self, plan: ValvePlan) -> Dict[str, Any]:
        if not self.base_url:
            return {'status': 'skipped', 'reason': 'no_valve_controller_configured', 'mock': True}

        import httpx
        headers = {'Content-Type': 'application/json'}
        if self.token:
            headers['Authorization'] = f'Bearer {self.token}'

        try:
            with httpx.Client(timeout=self.timeout) as client:
                resp = client.post(
                    f'{self.base_url}/valves/open',
                    json=plan.to_dict(),
                    headers=headers
                )
                if resp.status_code in (200, 201, 202):
                    logger.info(f"[ACTUATION] Valve command accepted: {plan.valve_id or 'default'}")
                    return {'status': 'commanded', 'code': resp.status_code, 'mock': False}
                logger.error(f"[ACTUATION] Valve controller rejected the command: {resp.status_code} {resp.text[:200]}")
                return {'status': 'failed', 'code': resp.status_code, 'detail': resp.text[:300], 'mock': False}
        except Exception as exc:
            logger.exception(f"[ACTUATION] Valve controller unreachable: {exc}")
            return {'status': 'failed', 'error': str(exc), 'mock': True}


def get_valve_controller() -> ValveController:
    """Return the configured controller (simulation when none is configured)."""
    if settings.VALVE_CONTROLLER_URL:
        return HttpValveController()
    return SimulatedValveController()
