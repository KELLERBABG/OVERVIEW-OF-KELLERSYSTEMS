"""SQLite / Postgres Database Engine with WAL Mode, Connection Pooling and Seed Data."""
import secrets
from datetime import datetime, timedelta, timezone
from typing import Generator
from sqlalchemy import event
from sqlmodel import SQLModel, create_engine, Session, select
from ..core.config import settings
from ..core.logger import logger
from ..core.security import hash_password
from ..core.models import (
    UserModel,
    FieldModel,
    SensorModel,
    TelemetryReadingModel,
    IrrigationEventModel
)

DATABASE_URL = settings.DATABASE_URL

# Chat ids that older seed revisions assigned to every field. They are placeholders,
# not real Telegram chat ids, and must never be shared between two fields.
LEGACY_PLACEHOLDER_CHAT_IDS = {'123456789'}

connect_args = {'check_same_thread': False, 'timeout': 15} if 'sqlite' in DATABASE_URL else {}
engine = create_engine(
    DATABASE_URL,
    echo=False,
    connect_args=connect_args,
    pool_pre_ping=True
)

# Enable WAL Mode and busy timeout on SQLite to eliminate 'database is locked' errors
if 'sqlite' in DATABASE_URL:
    @event.listens_for(engine, 'connect')
    def set_sqlite_pragma(dbapi_connection, connection_record):
        cursor = dbapi_connection.cursor()
        try:
            cursor.execute('PRAGMA journal_mode=WAL')
            cursor.execute('PRAGMA synchronous=NORMAL')
            cursor.execute('PRAGMA busy_timeout=5000')
        finally:
            cursor.close()

def _run_migrations():
    """Ensure newly added columns exist in existing databases without breaking."""
    try:
        with engine.connect() as conn:
            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(fields)")
                    cols = {row[1] for row in cursor.fetchall()}
                    if cols and 'paused_until' not in cols:
                        logger.info("[MIGRATION] Adding missing column 'paused_until' to 'fields' table")
                        cursor.execute("ALTER TABLE fields ADD COLUMN paused_until DATETIME DEFAULT NULL")
                        raw_conn.commit()
                    if cols and 'farmer_chat_id' not in cols:
                        logger.info("[MIGRATION] Adding missing column 'farmer_chat_id' to 'fields' table")
                        cursor.execute("ALTER TABLE fields ADD COLUMN farmer_chat_id TEXT DEFAULT '123456789'")
                        raw_conn.commit()
                    if cols and 'user_id' not in cols:
                        logger.info("[MIGRATION] Adding missing column 'user_id' to 'fields' table")
                        cursor.execute("ALTER TABLE fields ADD COLUMN user_id INTEGER DEFAULT NULL")
                        raw_conn.commit()
                    if cols and 'invite_token' not in cols:
                        logger.info("[MIGRATION] Adding missing column 'invite_token' to 'fields' table")
                        cursor.execute("ALTER TABLE fields ADD COLUMN invite_token TEXT DEFAULT NULL")
                        raw_conn.commit()
                    if cols and 'timezone' not in cols:
                        logger.info("[MIGRATION] Adding missing column 'timezone' to 'fields' table")
                        cursor.execute("ALTER TABLE fields ADD COLUMN timezone TEXT DEFAULT 'Europe/Berlin'")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(users)")
                    user_cols = {row[1] for row in cursor.fetchall()}
                    if user_cols and 'token_version' not in user_cols:
                        logger.info("[MIGRATION] Adding missing column 'token_version' to 'users' table")
                        cursor.execute("ALTER TABLE users ADD COLUMN token_version INTEGER DEFAULT 1")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(fields)")
                    field_cols = {row[1] for row in cursor.fetchall()}
                    for column, ddl in (
                        ('language', "TEXT DEFAULT 'en'"),
                        ('quiet_hours_start', 'TEXT DEFAULT NULL'),
                        ('quiet_hours_end', 'TEXT DEFAULT NULL'),
                        ('digest_enabled', 'BOOLEAN DEFAULT 0'),
                        ('digest_day', 'INTEGER DEFAULT 0'),
                        ('digest_hour', 'INTEGER DEFAULT 8'),
                        ('escalation_phone', 'TEXT DEFAULT NULL')
                    ):
                        if field_cols and column not in field_cols:
                            logger.info(f"[MIGRATION] Adding missing column '{column}' to 'fields' table")
                            cursor.execute(f"ALTER TABLE fields ADD COLUMN {column} {ddl}")
                            raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(sensors)")
                    sensor_cols = {row[1] for row in cursor.fetchall()}
                    if sensor_cols and 'zone' not in sensor_cols:
                        logger.info("[MIGRATION] Adding missing column 'zone' to 'sensors' table")
                        cursor.execute("ALTER TABLE sensors ADD COLUMN zone TEXT DEFAULT NULL")
                        cursor.execute("CREATE INDEX IF NOT EXISTS ix_sensors_zone ON sensors (zone)")
                        raw_conn.commit()
                    for column, ddl in (
                        ('moisture_15cm_pct', 'REAL'),
                        ('moisture_30cm_pct', 'REAL'),
                        ('moisture_60cm_pct', 'REAL'),
                        ('soil_ec_us_cm', 'REAL')
                    ):
                        if column not in sensor_cols:
                            logger.info(f"[MIGRATION] Adding missing column '{column}' to 'sensors' table")
                            cursor.execute(f"ALTER TABLE sensors ADD COLUMN {column} {ddl}")
                            raw_conn.commit()
                    if 'deep_percolation' not in sensor_cols:
                        logger.info("[MIGRATION] Adding missing column 'deep_percolation' to 'sensors' table")
                        cursor.execute("ALTER TABLE sensors ADD COLUMN deep_percolation BOOLEAN DEFAULT 0")
                        raw_conn.commit()
                    if 'valve_id' not in sensor_cols:
                        logger.info("[MIGRATION] Adding missing column 'valve_id' to 'sensors' table")
                        cursor.execute("ALTER TABLE sensors ADD COLUMN valve_id TEXT DEFAULT NULL")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(advisory_alerts)")
                    alert_cols = {row[1] for row in cursor.fetchall()}
                    if alert_cols and 'recommended_gross_mm' not in alert_cols:
                        logger.info("[MIGRATION] Adding missing column 'recommended_gross_mm' to 'advisory_alerts' table")
                        cursor.execute("ALTER TABLE advisory_alerts ADD COLUMN recommended_gross_mm REAL DEFAULT 0.0")
                        raw_conn.commit()
                    if alert_cols and 'zone' not in alert_cols:
                        logger.info("[MIGRATION] Adding missing column 'zone' to 'advisory_alerts' table")
                        cursor.execute("ALTER TABLE advisory_alerts ADD COLUMN zone TEXT DEFAULT NULL")
                        cursor.execute("CREATE INDEX IF NOT EXISTS ix_advisory_alerts_zone ON advisory_alerts (zone)")
                        raw_conn.commit()
                    if alert_cols and 'urgency' not in alert_cols:
                        logger.info("[MIGRATION] Adding missing column 'urgency' to 'advisory_alerts' table")
                        cursor.execute("ALTER TABLE advisory_alerts ADD COLUMN urgency TEXT DEFAULT 'NONE'")
                        cursor.execute("CREATE INDEX IF NOT EXISTS ix_advisory_alerts_urgency ON advisory_alerts (urgency)")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(irrigation_events)")
                    event_cols = {row[1] for row in cursor.fetchall()}
                    if event_cols and 'zone' not in event_cols:
                        logger.info("[MIGRATION] Adding missing column 'zone' to 'irrigation_events' table")
                        cursor.execute("ALTER TABLE irrigation_events ADD COLUMN zone TEXT DEFAULT NULL")
                        cursor.execute("CREATE INDEX IF NOT EXISTS ix_irrigation_events_zone ON irrigation_events (zone)")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(telemetry_readings)")
                    telemetry_cols = {row[1] for row in cursor.fetchall()}
                    for column, ddl in (
                        ('moisture_15cm_pct', 'REAL'),
                        ('moisture_30cm_pct', 'REAL'),
                        ('moisture_60cm_pct', 'REAL'),
                        ('soil_ec_us_cm', 'REAL')
                    ):
                        if telemetry_cols and column not in telemetry_cols:
                            logger.info(f"[MIGRATION] Adding missing column '{column}' to 'telemetry_readings' table")
                            cursor.execute(f"ALTER TABLE telemetry_readings ADD COLUMN {column} {ddl}")
                            raw_conn.commit()
                    if telemetry_cols and 'deep_percolation' not in telemetry_cols:
                        logger.info("[MIGRATION] Adding missing column 'deep_percolation' to 'telemetry_readings' table")
                        cursor.execute("ALTER TABLE telemetry_readings ADD COLUMN deep_percolation BOOLEAN DEFAULT 0")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(telemetry_readings)")
                    telemetry_cols = {row[1] for row in cursor.fetchall()}
                    if telemetry_cols and 'source' not in telemetry_cols:
                        logger.info("[MIGRATION] Adding missing column 'source' to 'telemetry_readings' table")
                        cursor.execute("ALTER TABLE telemetry_readings ADD COLUMN source TEXT DEFAULT 'ttn'")
                        raw_conn.commit()
                finally:
                    cursor.close()

            if 'sqlite' in str(engine.url):
                raw_conn = conn.connection
                cursor = raw_conn.cursor()
                try:
                    cursor.execute("PRAGMA table_info(advisory_alerts)")
                    alert_cols = {row[1] for row in cursor.fetchall()}
                    if alert_cols and 'recommended_gross_mm' not in alert_cols:
                        logger.info("[MIGRATION] Adding missing column 'recommended_gross_mm' to 'advisory_alerts' table")
                        cursor.execute("ALTER TABLE advisory_alerts ADD COLUMN recommended_gross_mm REAL DEFAULT 0.0")
                        raw_conn.commit()
                finally:
                    cursor.close()
    except Exception as e:
        logger.warning(f"[MIGRATION WARNING] Auto-migration check encountered an issue: {e}")

def create_db_and_tables():
    SQLModel.metadata.create_all(engine)
    _run_migrations()
    seed_initial_data()

def get_session() -> Generator[Session, None, None]:
    with Session(engine) as session:
        yield session

def seed_initial_data():
    with Session(engine) as session:
        # Seed default admin user if not exists
        admin_user = session.exec(select(UserModel).where(UserModel.email == 'admin@terraflux.ag')).first()
        if not admin_user:
            admin_user = UserModel(
                email='admin@terraflux.ag',
                hashed_password=hash_password('kaiserstuhl2026'),
                full_name='TerraFlux Administrator',
                role='admin',
                is_active=True
            )
            session.add(admin_user)
            session.commit()
            session.refresh(admin_user)
            logger.info("[SEED] Initialized default admin user admin@terraflux.ag")

        # Check if fields already exist; backfill missing user_id or invite_token
        existing_fields = session.exec(select(FieldModel)).all()
        if existing_fields:
            default_tokens = ['kaiser_alpha', 'wachau_gala']
            changed_any = False
            for i, f in enumerate(existing_fields):
                if f.user_id is None:
                    f.user_id = admin_user.id
                    changed_any = True
                if not f.invite_token:
                    f.invite_token = default_tokens[i] if i < len(default_tokens) else secrets.token_hex(4)
                    changed_any = True
                if not getattr(f, 'timezone', None):
                    f.timezone = 'Europe/Berlin'
                    changed_any = True
                if changed_any:
                    session.add(f)
            if changed_any:
                session.commit()
                logger.info("[SEED] Backfilled user_id, timezone and invite_tokens for existing fields")

            # Repair legacy Telegram chat-id collisions.
            # Older seeds gave every field the same placeholder chat id ('123456789'), so
            # advisories for one block were delivered to the chat of another one.
            # Only the legacy placeholder is repaired - chat ids that a grower actually
            # linked through /start onboarding are never touched, because a grower may
            # legitimately manage several blocks from one chat.
            collision_fixed = False
            for f in existing_fields:
                if f.farmer_chat_id in LEGACY_PLACEHOLDER_CHAT_IDS:
                    logger.info(
                        f"[SEED] Field '{f.name}' still carries the legacy placeholder chat id "
                        f"'{f.farmer_chat_id}' - assigning a unique placeholder"
                    )
                    f.farmer_chat_id = str(900000000 + int(f.id or 0))
                    session.add(f)
                    collision_fixed = True
            if collision_fixed:
                session.commit()
            return

        # Seed Field 1: Vineyard Block A (Baden, Germany)
        vineyard = FieldModel(
            user_id=admin_user.id,
            invite_token='kaiser_alpha',
            name='Kaiserstuhl Vineyard (Block A - Spätburgunder)',
            location_name='Ihringen am Kaiserstuhl, Baden-Württemberg',
            latitude=48.0438,
            longitude=7.6481,
            area_ha=14.5,
            crop_key='grapevine',
            growth_stage='mid_season',
            soil_key='sandy_loam',
            timezone='Europe/Berlin',
            drip_emitter_spacing_m=0.5,
            drip_emitter_lph=2.0,
            drip_line_spacing_m=2.0,
            irrigation_efficiency=0.90,
            farmer_name='Stefan Becker',
            farmer_phone='+4915123456789',
            farmer_chat_id='900000001',
            daily_alert_hour=19,
            active=True
        )
        session.add(vineyard)
        session.commit()
        session.refresh(vineyard)

        # Seed Sensor 1
        sensor1 = SensorModel(
            deveui='2CF7F1C044300001',
            name='Probe-Vineyard-North',
            field_id=vineyard.id,
            model_type='Dragino-LSE01',
            install_depth_cm=40,
            battery_volts=3.64,
            last_moisture_pct=19.4,
            last_temp_c=22.8,
            last_seen=datetime.now(timezone.utc),
            is_simulated=True
        )
        session.add(sensor1)
        session.commit()
        session.refresh(sensor1)

        # Seed historical telemetry for past 7 days
        base_time = datetime.now(timezone.utc) - timedelta(days=7)
        for d in range(7):
            moisture = 23.5 - (d * 0.6) # gradual drying
            for h in (8, 14, 20):
                t_point = base_time + timedelta(days=d, hours=h)
                reading = TelemetryReadingModel(
                    sensor_id=sensor1.id,
                    timestamp=t_point,
                    soil_moisture_pct=round(moisture, 1),
                    soil_temp_c=round(19.0 + (h - 8) * 0.8, 1),
                    battery_volts=3.65,
                    rssi=-78,
                    snr=8.5
                )
                session.add(reading)

        # Seed a past irrigation event
        past_event = IrrigationEventModel(
            field_id=vineyard.id,
            timestamp=datetime.now(timezone.utc) - timedelta(days=8),
            depth_applied_mm=12.0,
            duration_minutes=135,
            source='whatsapp_reply',
            notes='Scheduled mid-season top-up'
        )
        session.add(past_event)

        # Seed Field 2: Apple Orchard (South Tyrol / Wachau)
        orchard = FieldModel(
            user_id=admin_user.id,
            invite_token='wachau_gala',
            name='Wachau Apple Orchard (Gala)',
            location_name='Spitz an der Donau, Austria',
            latitude=48.3667,
            longitude=15.4167,
            area_ha=8.0,
            crop_key='apple',
            growth_stage='mid_season',
            soil_key='silt_loam',
            timezone='Europe/Vienna',
            drip_emitter_spacing_m=0.4,
            drip_emitter_lph=2.2,
            drip_line_spacing_m=3.0,
            irrigation_efficiency=0.92,
            farmer_name='Markus Gruber',
            farmer_phone='+436601234567',
            farmer_chat_id='900000002',
            daily_alert_hour=19,
            active=True
        )
        session.add(orchard)
        session.commit()
        session.refresh(orchard)

        sensor2 = SensorModel(
            deveui='2CF7F1C044300002',
            name='Probe-Orchard-East',
            field_id=orchard.id,
            model_type='SenseCAP-Soil',
            install_depth_cm=50,
            battery_volts=3.68,
            last_moisture_pct=26.2,
            last_temp_c=19.5,
            last_seen=datetime.now(timezone.utc),
            is_simulated=True
        )
        session.add(sensor2)
        session.commit()

        logger.info('Database seeded with initial fields, sensors, and telemetry.')
