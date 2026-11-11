"""Database models and domain entities for TerraFlux (FieldSense Engine)."""
import secrets
from datetime import datetime, timezone
from typing import Optional, List
from sqlalchemy import Index
from sqlmodel import SQLModel, Field, Relationship

class UserModel(SQLModel, table=True):
    __tablename__ = 'users'

    id: Optional[int] = Field(default=None, primary_key=True)
    email: str = Field(unique=True, index=True)
    hashed_password: str
    full_name: str = Field(default='Grower')
    role: str = Field(default='grower') # 'admin' or 'grower'
    is_active: bool = Field(default=True)
    # Incremented on logout / credential change. Tokens carrying an older version are
    # rejected, which gives the API a server-side session revocation.
    token_version: int = Field(default=1)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    fields: List['FieldModel'] = Relationship(back_populates='owner')


class FieldModel(SQLModel, table=True):
    __tablename__ = 'fields'
    __table_args__ = (
        Index('ix_field_phone', 'farmer_phone'),
        Index('ix_field_active_hour', 'active', 'daily_alert_hour'),
        Index('ix_field_invite_token', 'invite_token'),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    user_id: Optional[int] = Field(default=None, foreign_key='users.id', index=True)
    invite_token: str = Field(default_factory=lambda: secrets.token_hex(4), index=True)
    name: str = Field(index=True)
    location_name: str = Field(default='Kaiserstuhl, Baden')
    latitude: float = Field(default=48.0833)
    longitude: float = Field(default=7.6833)
    area_ha: float = Field(default=12.5)
    crop_key: str = Field(default='grapevine') # grapevine, maize, apple, olive
    growth_stage: str = Field(default='mid_season') # initial, development, mid_season, late_season
    soil_key: str = Field(default='sandy_loam') # sandy_loam, clay_loam, silt_loam, sand
    
    # Irrigation System Engineering Parameters
    drip_emitter_spacing_m: float = Field(default=0.5) # Emitter every 50cm
    drip_emitter_lph: float = Field(default=2.0) # 2.0 Liters per hour per emitter
    drip_line_spacing_m: float = Field(default=2.0) # Row spacing (e.g. 2m for vineyards)
    irrigation_efficiency: float = Field(default=0.90) # 90% application efficiency for drip

    # Notification Channels (WhatsApp / Telegram)
    farmer_phone: str = Field(default='+4915123456789', index=True)
    # Telegram chat_id of the grower. Deliberately None by default: a shared hardcoded
    # default would route every advisory to one chat and cross-wire multi-block growers.
    farmer_chat_id: Optional[str] = Field(default=None, index=True)
    farmer_name: str = Field(default='Stefan Becker')
    timezone: str = Field(default='Europe/Berlin') # IANA Timezone (e.g. Europe/Berlin, Europe/Vienna)
    daily_alert_hour: int = Field(default=19) # 19:00 (7 PM daily advisory in local farm time)
    # Grower facing language of the messages ('en' or 'de')
    language: str = Field(default='en')
    # Quiet hours (local farm time, 'HH:MM'). No advisories are sent inside the window.
    quiet_hours_start: Optional[str] = None
    quiet_hours_end: Optional[str] = None
    # Weekly digest (Monday = 0 .. Sunday = 6)
    digest_enabled: bool = Field(default=False)
    digest_day: int = Field(default=0)
    digest_hour: int = Field(default=8)
    # Second contact that is notified when a critical advisory stays unconfirmed
    escalation_phone: Optional[str] = Field(default=None)
    active: bool = Field(default=True, index=True)
    paused_until: Optional[datetime] = Field(default=None)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    # Relationships
    owner: Optional[UserModel] = Relationship(back_populates='fields')
    sensors: List['SensorModel'] = Relationship(back_populates='field')
    events: List['IrrigationEventModel'] = Relationship(back_populates='field')
    alerts: List['AdvisoryAlertModel'] = Relationship(back_populates='field')

    @property
    def application_rate_mm_h(self) -> float:
        """NET application rate in mm/hour (liters per m^2 per hour).

        This is the gross emitter rate reduced by the irrigation efficiency, i.e. the
        rate at which water actually reaches the root zone.
        """
        return self.gross_application_rate_mm_h * self.irrigation_efficiency

    @property
    def gross_application_rate_mm_h(self) -> float:
        """GROSS application rate in mm/hour (water leaving the emitters)."""
        wetted_area_per_emitter = self.drip_emitter_spacing_m * self.drip_line_spacing_m
        if wetted_area_per_emitter <= 0:
            return 2.0
        return self.drip_emitter_lph / wetted_area_per_emitter


class SensorModel(SQLModel, table=True):
    __tablename__ = 'sensors'

    id: Optional[int] = Field(default=None, primary_key=True)
    deveui: str = Field(index=True, unique=True)
    name: str = Field(default='Probe-Alpha-01')
    field_id: Optional[int] = Field(default=None, foreign_key='fields.id', index=True)
    model_type: str = Field(default='Dragino-LSE01') # Dragino-LSE01, Cayenne-LPP, SenseCAP, DIY-ESP32
    install_depth_cm: int = Field(default=40)
    # Irrigation zone this probe represents (e.g. 'A1', 'B2'). Probes without a zone
    # belong to the block as a whole.
    zone: Optional[str] = Field(default=None, index=True)
    # Physical valve this zone is fed by (e.g. 'V1'), used in the grower message.
    valve_id: Optional[str] = Field(default=None)
    # Multi-depth profile. `last_moisture_pct` always holds the effective root-zone
    # value (weighted 25/50/25 over the depths that are actually reported).
    moisture_15cm_pct: Optional[float] = None
    moisture_30cm_pct: Optional[float] = None
    moisture_60cm_pct: Optional[float] = None
    soil_ec_us_cm: Optional[float] = None
    deep_percolation: bool = Field(default=False)
    battery_volts: float = Field(default=3.65)
    last_moisture_pct: float = Field(default=22.5)
    last_temp_c: float = Field(default=21.0)
    last_seen: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    is_simulated: bool = Field(default=False)

    field: Optional[FieldModel] = Relationship(back_populates='sensors')
    telemetry: List['TelemetryReadingModel'] = Relationship(back_populates='sensor')


class TelemetryReadingModel(SQLModel, table=True):
    __tablename__ = 'telemetry_readings'
    __table_args__ = (
        Index('ix_telemetry_sensor_ts', 'sensor_id', 'timestamp'),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    sensor_id: int = Field(foreign_key='sensors.id', index=True)
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc), index=True)
    soil_moisture_pct: float # Volumetric Water Content % (VWC)
    soil_temp_c: float
    battery_volts: float
    raw_adc: Optional[int] = None
    rssi: Optional[int] = None
    snr: Optional[float] = None
    # Provenance of the reading: 'ttn' (real LoRaWAN uplink), 'simulation' (fast-forward
    # playground), 'manual' (operator entry). Synthetic data must never be mixed silently
    # with physical probe data.
    source: str = Field(default='ttn', index=True)
    # Multi-depth profile (optional - only reported by multi-depth probes)
    moisture_15cm_pct: Optional[float] = None
    moisture_30cm_pct: Optional[float] = None
    moisture_60cm_pct: Optional[float] = None
    soil_ec_us_cm: Optional[float] = None
    # True when the 60 cm layer rose by more than 2 % - water is leaving the root zone.
    deep_percolation: bool = Field(default=False)

    sensor: Optional[SensorModel] = Relationship(back_populates='telemetry')


class IrrigationEventModel(SQLModel, table=True):
    __tablename__ = 'irrigation_events'

    id: Optional[int] = Field(default=None, primary_key=True)
    field_id: int = Field(foreign_key='fields.id', index=True)
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc), index=True)
    # NET depth that reached the root zone (mm). The gross volume that had to be run
    # through the emitters is recorded in `notes` together with the application
    # efficiency, so the audit trail keeps both numbers.
    depth_applied_mm: float
    duration_minutes: int
    # Irrigation zone the event belongs to (e.g. 'A1'), so the history shows which
    # part of the block was actually watered.
    zone: Optional[str] = Field(default=None, index=True)
    source: str = Field(default='whatsapp_reply') # whatsapp_reply, auto_schedule, manual_ui
    notes: Optional[str] = None

    field: Optional[FieldModel] = Relationship(back_populates='events')


class ActuationEventModel(SQLModel, table=True):
    """Audit trail of valve plans and valve commands."""
    __tablename__ = 'actuation_events'

    id: Optional[int] = Field(default=None, primary_key=True)
    field_id: Optional[int] = Field(default=None, foreign_key='fields.id', index=True)
    alert_id: Optional[int] = Field(default=None, foreign_key='advisory_alerts.id', index=True)
    zone: Optional[str] = Field(default=None, index=True)
    valve_id: Optional[str] = Field(default=None)
    status: str = Field(default='planned', index=True)  # planned | commanded | simulated | confirmed | failed
    start_time: Optional[str] = None
    duration_minutes: int = 0
    gross_depth_mm: float = 0.0
    net_depth_mm: float = 0.0
    detail: Optional[str] = None
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc), index=True)


class DispatchLogModel(SQLModel, table=True):
    """Audit trail of every outbound advisory attempt.

    Each row records one channel attempt (Telegram / WhatsApp), its outcome and the
    number of attempts that were needed. Failed deliveries stay visible instead of
    disappearing into the log file.
    """
    __tablename__ = 'dispatch_logs'

    id: Optional[int] = Field(default=None, primary_key=True)
    field_id: Optional[int] = Field(default=None, foreign_key='fields.id', index=True)
    alert_id: Optional[int] = Field(default=None, foreign_key='advisory_alerts.id', index=True)
    channel: str = Field(default='telegram', index=True)  # telegram | whatsapp
    target: str = Field(default='')
    status: str = Field(default='sent', index=True)  # sent | simulated | failed | skipped
    detail: Optional[str] = None
    attempts: int = Field(default=1)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc), index=True)


class SchedulerLockModel(SQLModel, table=True):
    """Cooperative lease used to elect a single dispatch leader.

    With more than one API worker (or one API worker plus a dedicated scheduler
    container) only the holder of a valid lease may send grower notifications.
    """
    __tablename__ = 'scheduler_locks'

    id: Optional[int] = Field(default=None, primary_key=True)
    name: str = Field(index=True, unique=True)
    holder: str = Field(default='')
    acquired_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    expires_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class AdvisoryAlertModel(SQLModel, table=True):
    __tablename__ = 'advisory_alerts'
    __table_args__ = (
        Index('ix_alerts_field_ts', 'field_id', 'timestamp'),
    )

    id: Optional[int] = Field(default=None, primary_key=True)
    field_id: int = Field(foreign_key='fields.id', index=True)
    timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc), index=True)
    soil_moisture_pct: float
    threshold_pct: float
    forecast_et0_mm: float
    forecast_rain_mm: float
    recommended_depth_mm: float   # NET depth that must reach the root zone (mm)
    recommended_gross_mm: float = Field(default=0.0)  # GROSS depth to run through the emitters (mm)
    recommended_duration_min: int
    recommended_start_time: str = Field(default='06:00')
    # Irrigation zone the advisory refers to (None = whole block)
    zone: Optional[str] = Field(default=None, index=True)
    urgency: str = Field(default='NONE', index=True)  # NONE | LOW | OPTIMAL | HIGH | CRITICAL
    status: str = Field(default='PENDING') # PENDING, ACCEPTED, SNOOZED, REJECTED, AUTO_LOGGED, ESCALATED
    message_text: str

    field: Optional[FieldModel] = Relationship(back_populates='alerts')
