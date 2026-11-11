"""Actuation planning and the audit trail of valve commands."""
from typing import Any, Dict, List, Optional

from sqlmodel import Session

from ..core.advisor import AdvisoryDecision
from ..core.logger import logger
from ..core.models import ActuationEventModel, AdvisoryAlertModel, FieldModel
from .controller import ValvePlan, get_valve_controller


def build_plan(field: FieldModel, decision: AdvisoryDecision) -> Optional[ValvePlan]:
    """Translate an advisory decision into an executable valve plan."""
    if not decision.should_irrigate or decision.duration_minutes <= 0:
        return None

    return ValvePlan(
        field_id=field.id,
        field_name=field.name,
        zone=decision.zone,
        valve_id=decision.valve_id,
        start_time=decision.optimal_start_time,
        duration_minutes=decision.duration_minutes,
        gross_depth_mm=decision.gross_depth_mm,
        net_depth_mm=decision.target_depth_mm,
        application_rate_mm_h=round(field.gross_application_rate_mm_h, 2),
        irrigation_efficiency=field.irrigation_efficiency,
        reason=decision.reason
    )


def record_event(
    session: Session,
    plan: ValvePlan,
    status: str,
    detail: Optional[str] = None,
    alert_id: Optional[int] = None
) -> ActuationEventModel:
    """Persist one actuation state change (planned / commanded / simulated / failed)."""
    event = ActuationEventModel(
        field_id=plan.field_id,
        alert_id=alert_id,
        zone=plan.zone,
        valve_id=plan.valve_id,
        status=status,
        start_time=plan.start_time,
        duration_minutes=plan.duration_minutes,
        gross_depth_mm=plan.gross_depth_mm,
        net_depth_mm=plan.net_depth_mm,
        detail=detail
    )
    session.add(event)
    session.commit()
    session.refresh(event)
    return event


def execute_plan(
    session: Session,
    plan: ValvePlan,
    alert: Optional[AdvisoryAlertModel] = None
) -> Dict[str, Any]:
    """Send the plan to the valve controller and store the outcome."""
    controller = get_valve_controller()
    outcome = controller.open_valve(plan)
    status = outcome.get('status', 'unknown')
    detail = outcome.get('detail') or outcome.get('error') or outcome.get('reason')
    event = record_event(
        session=session,
        plan=plan,
        status=status,
        detail=detail,
        alert_id=alert.id if alert else None
    )
    logger.info(
        f"[ACTUATION] {plan.field_name} zone={plan.zone or '-'} valve={plan.valve_id or '-'} "
        f"-> {status}"
    )
    return {'plan': plan.to_dict(), 'outcome': outcome, 'event_id': event.id, 'status': status}


def list_events(session: Session, field_id: int, limit: int = 50) -> List[ActuationEventModel]:
    from sqlmodel import select
    return session.exec(
        select(ActuationEventModel)
        .where(ActuationEventModel.field_id == field_id)
        .order_by(ActuationEventModel.created_at.desc())
        .limit(limit)
    ).all()
