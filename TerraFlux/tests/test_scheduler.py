from src.api.database import create_db_and_tables
from src.api.scheduler import DailyAdvisoryScheduler

create_db_and_tables()

def test_hourly_scheduler_dispatch():
    """Verify that hourly dispatch executes without errors for scheduled hour."""
    # Run dispatch for hour 19 (default hour of seeded fields) with force=True
    dispatched = DailyAdvisoryScheduler.run_hourly_dispatch(19, force=True)
    assert dispatched >= 1

def test_hourly_scheduler_non_matching_hour():
    """Verify that non-matching hours do not dispatch."""
    dispatched = DailyAdvisoryScheduler.run_hourly_dispatch(3) # 03:00 AM
    assert dispatched == 0

def test_hourly_scheduler_skips_paused_fields():
    """Verify that fields paused into the future are skipped."""
    from datetime import datetime, timezone, timedelta
    from sqlmodel import Session, select
    from src.api.database import engine
    from src.core.models import FieldModel

    with Session(engine) as session:
        field = session.exec(select(FieldModel).where(FieldModel.daily_alert_hour == 19)).first()
        assert field is not None
        # Pause field
        field.paused_until = datetime.now(timezone.utc) + timedelta(days=2)
        session.add(field)
        session.commit()

    dispatched = DailyAdvisoryScheduler.run_hourly_dispatch(19)
    # The paused field should be skipped
    assert dispatched == 0

    # Reset paused_until
    with Session(engine) as session:
        field = session.exec(select(FieldModel).where(FieldModel.daily_alert_hour == 19)).first()
        field.paused_until = None
        session.add(field)
        session.commit()

def test_hourly_scheduler_timezone_aware():
    """Verify that live dispatch with target_hour=None evaluates fields based on local farm timezone."""
    from zoneinfo import ZoneInfo
    from datetime import datetime
    from sqlmodel import Session, select
    from src.api.database import engine
    from src.core.models import FieldModel

    with Session(engine) as session:
        # Create a test field whose daily_alert_hour matches current local hour in Europe/Berlin
        berlin_hour = datetime.now(ZoneInfo('Europe/Berlin')).hour
        tz_field = FieldModel(
            name="Timezone Test Vineyard",
            crop_key="grapevine",
            timezone="Europe/Berlin",
            daily_alert_hour=berlin_hour,
            active=True
        )
        session.add(tz_field)
        session.commit()
        session.refresh(tz_field)
        tz_field_id = tz_field.id

    # Call run_hourly_dispatch without target_hour (production timezone-aware mode)
    dispatched = DailyAdvisoryScheduler.run_hourly_dispatch(target_hour=None, force=True)
    assert dispatched >= 1

    # Cleanup test field and its generated alerts
    with Session(engine) as session:
        from src.core.models import AdvisoryAlertModel
        alerts = session.exec(select(AdvisoryAlertModel).where(AdvisoryAlertModel.field_id == tz_field_id)).all()
        for a in alerts:
            session.delete(a)
        f = session.get(FieldModel, tz_field_id)
        if f:
            session.delete(f)
        session.commit()


