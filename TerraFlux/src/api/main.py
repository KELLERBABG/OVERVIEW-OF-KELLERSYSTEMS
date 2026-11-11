"""FieldSense Irrigation — FastAPI Production Server & Dashboard API."""
import uuid
import pathlib
from datetime import datetime, timezone
from contextlib import asynccontextmanager
from typing import List, Dict, Any, Optional
from fastapi import FastAPI, Depends, HTTPException, Query, Request, Response, Header, status
from fastapi.responses import HTMLResponse
from fastapi.middleware.cors import CORSMiddleware
from sqlmodel import Session, select, text

from pydantic import BaseModel
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials

from .database import create_db_and_tables, get_session
from .scheduler import DailyAdvisoryScheduler
from ..core.config import settings
from ..core.logger import logger
from ..core.security import hash_password, verify_password, create_access_token, decode_access_token
from ..core.rate_limit import auth_limiter, api_limiter, webhook_limiter
from ..core.models import (
    UserModel,
    FieldModel,
    SensorModel,
    TelemetryReadingModel,
    IrrigationEventModel,
    AdvisoryAlertModel,
    DispatchLogModel
)
from ..core.water_balance import WaterBalanceEngine, CROPS_CONFIG, SOILS_CONFIG
from ..core.advisor import AgronomicAdvisor, hours_since_reading
from ..ingestion.weather import WeatherService
from ..ingestion.lorawan_decoder import LoRaWANDecoder
from ..bot.formatter import WhatsAppFormatter
from ..bot.handler import WhatsAppCommandHandler
from ..bot.dispatch import AdvisoryDispatcher
from ..bot.twilio_client import TwilioWhatsAppClient
from ..bot.i18n import normalize_language
from ..actuation import build_plan, execute_plan as execute_actuation_plan
from ..actuation import list_events as list_actuation_events
from ..actuation import record_event as record_actuation_event
from ..simulation.sensor_sim import SoilSensorSimulator
from ..simulation.bom_economics import BOMEconomicsModel

# Security & Auth Bearer Scheme
http_bearer = HTTPBearer(auto_error=False)

def client_key(request: Request) -> str:
    """Best-effort client identifier for rate limiting (honours X-Forwarded-For)."""
    forwarded = request.headers.get('x-forwarded-for')
    if forwarded:
        return forwarded.split(',')[0].strip()
    if request.client and request.client.host:
        return request.client.host
    return 'unknown'

def validate_password_strength(password: str) -> Optional[str]:
    """Return an error message when the password does not meet the policy, else None."""
    if len(password or '') < settings.PASSWORD_MIN_LENGTH:
        return f'Password must be at least {settings.PASSWORD_MIN_LENGTH} characters long'
    if not any(c.isalpha() for c in password) or not any(c.isdigit() for c in password):
        return 'Password must contain at least one letter and one digit'
    return None

def enforce_rate_limit(limiter, key: str, limit: int, window_seconds: int, detail: str) -> None:
    """Raise HTTP 429 when the caller exceeded the limit."""
    allowed, retry_after = limiter.check(key, limit, window_seconds)
    if not allowed:
        logger.warning(f"[RATE LIMIT] {detail} | key={key} | retry_after={retry_after}s")
        raise HTTPException(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail=f'{detail}. Retry in {retry_after} seconds.',
            headers={'Retry-After': str(retry_after)}
        )

def get_current_user_optional(
    credentials: Optional[HTTPAuthorizationCredentials] = Depends(http_bearer),
    session: Session = Depends(get_session)
) -> Optional[UserModel]:
    if not credentials:
        return None
    token = credentials.credentials
    payload = decode_access_token(token)
    if not payload:
        return None
    user_id = payload.get('sub')
    if not user_id:
        return None
    user = session.get(UserModel, user_id)
    if not user or not user.is_active:
        return None
    # Revoked tokens (logout / credential change) carry an outdated version.
    if int(payload.get('ver', 1)) != int(user.token_version):
        return None
    return user

def get_current_user(
    credentials: Optional[HTTPAuthorizationCredentials] = Depends(http_bearer),
    session: Session = Depends(get_session)
) -> UserModel:
    if not credentials:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail='Authentication credentials required',
            headers={'WWW-Authenticate': 'Bearer'}
        )
    token = credentials.credentials
    payload = decode_access_token(token)
    if not payload:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail='Invalid or expired authentication token',
            headers={'WWW-Authenticate': 'Bearer'}
        )
    user_id = payload.get('sub')
    user = session.get(UserModel, user_id)
    if not user or not user.is_active:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail='User not found or account is deactivated',
            headers={'WWW-Authenticate': 'Bearer'}
        )
    if int(payload.get('ver', 1)) != int(user.token_version):
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail='Token has been revoked - please sign in again',
            headers={'WWW-Authenticate': 'Bearer'}
        )
    return user

def require_field_access(field: FieldModel, current_user: UserModel) -> None:
    """Enforce tenant isolation for field-scoped resources.

    Administrators may access every field; growers may only access fields they own.
    """
    if current_user.role == 'admin':
        return
    if field.user_id != current_user.id:
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail='Permission denied to access this field'
        )

def validate_agronomy(crop_key: str, growth_stage: str, soil_key: str) -> None:
    """Reject unknown crops / growth stages / soils before they reach the agronomy engine."""
    if crop_key not in CROPS_CONFIG:
        raise HTTPException(
            status_code=400,
            detail=f"Unknown crop '{crop_key}'. Known crops: {', '.join(sorted(CROPS_CONFIG))}"
        )
    stages = CROPS_CONFIG[crop_key].get('stages', {})
    if growth_stage not in stages:
        raise HTTPException(
            status_code=400,
            detail=f"Unknown growth stage '{growth_stage}' for {crop_key}. Valid stages: {', '.join(sorted(stages))}"
        )
    if soil_key not in SOILS_CONFIG:
        raise HTTPException(
            status_code=400,
            detail=f"Unknown soil '{soil_key}'. Known soils: {', '.join(sorted(SOILS_CONFIG))}"
        )

def _sensor_payload(sensor: SensorModel) -> Dict[str, Any]:
    return {
        'id': sensor.id,
        'deveui': sensor.deveui,
        'name': sensor.name,
        'field_id': sensor.field_id,
        'model_type': sensor.model_type,
        'install_depth_cm': sensor.install_depth_cm,
        'zone': sensor.zone,
        'valve_id': sensor.valve_id,
        'last_moisture_pct': sensor.last_moisture_pct,
        'last_temp_c': sensor.last_temp_c,
        'battery_volts': sensor.battery_volts,
        'last_seen': sensor.last_seen.isoformat() if sensor.last_seen else None,
        'is_simulated': sensor.is_simulated,
        'profile': {
            'moisture_15cm_pct': sensor.moisture_15cm_pct,
            'moisture_30cm_pct': sensor.moisture_30cm_pct,
            'moisture_60cm_pct': sensor.moisture_60cm_pct,
            'soil_ec_us_cm': sensor.soil_ec_us_cm,
            'deep_percolation': sensor.deep_percolation
        }
    }

def _zone_payload(field: FieldModel, sensors: List[SensorModel]) -> List[Dict[str, Any]]:
    """Per-zone soil status derived from the probes that represent each zone."""
    zones: Dict[str, Dict[str, Any]] = {}
    for sensor in sensors:
        key = sensor.zone or 'BLOCK'
        status = WaterBalanceEngine.analyze_soil_status(
            current_moisture_pct=sensor.last_moisture_pct,
            crop_key=field.crop_key,
            growth_stage=field.growth_stage,
            soil_key=field.soil_key
        )
        zones[key] = {
            'zone': key,
            'sensor_id': sensor.id,
            'sensor_name': sensor.name,
            'moisture_pct': sensor.last_moisture_pct,
            'temperature_c': sensor.last_temp_c,
            'battery_volts': sensor.battery_volts,
            'threshold_pct': status.threshold_moisture_pct,
            'is_stressed': status.is_stressed,
            'last_seen': sensor.last_seen.isoformat() if sensor.last_seen else None,
            'profile': {
                'moisture_15cm_pct': sensor.moisture_15cm_pct,
                'moisture_30cm_pct': sensor.moisture_30cm_pct,
                'moisture_60cm_pct': sensor.moisture_60cm_pct,
                'soil_ec_us_cm': sensor.soil_ec_us_cm,
                'deep_percolation': sensor.deep_percolation
            }
        }
    return [zones[k] for k in sorted(zones)]

class RegisterRequest(BaseModel):
    """Self-serve grower registration payload.

    NOTE: the role is deliberately NOT client-controllable. Public registration always
    creates a 'grower' account; elevation to 'admin' requires an authenticated
    administrator calling POST /api/v1/admin/users/{user_id}/role.
    """
    email: str
    password: str
    full_name: Optional[str] = 'Grower'

class RoleUpdateRequest(BaseModel):
    """Administrative role assignment payload (admin-only endpoint)."""
    role: str

class FieldCreateRequest(BaseModel):
    """Payload for creating a new farm block."""
    name: str
    location_name: Optional[str] = None
    latitude: Optional[float] = None
    longitude: Optional[float] = None
    area_ha: float = 1.0
    crop_key: str = 'grapevine'
    growth_stage: str = 'mid_season'
    soil_key: str = 'sandy_loam'
    timezone: str = 'Europe/Berlin'
    daily_alert_hour: int = 19
    drip_emitter_spacing_m: float = 0.5
    drip_emitter_lph: float = 2.0
    drip_line_spacing_m: float = 2.0
    irrigation_efficiency: float = 0.90
    farmer_name: Optional[str] = None
    farmer_phone: Optional[str] = None
    farmer_chat_id: Optional[str] = None
    language: str = 'en'
    quiet_hours_start: Optional[str] = None
    quiet_hours_end: Optional[str] = None
    digest_enabled: bool = False
    digest_day: int = 0
    digest_hour: int = 8
    escalation_phone: Optional[str] = None
    user_id: Optional[int] = None  # administrators may create a block for another tenant

class FieldUpdateRequest(BaseModel):
    """Partial update of a farm block - every field is optional."""
    name: Optional[str] = None
    location_name: Optional[str] = None
    latitude: Optional[float] = None
    longitude: Optional[float] = None
    area_ha: Optional[float] = None
    crop_key: Optional[str] = None
    growth_stage: Optional[str] = None
    soil_key: Optional[str] = None
    timezone: Optional[str] = None
    daily_alert_hour: Optional[int] = None
    drip_emitter_spacing_m: Optional[float] = None
    drip_emitter_lph: Optional[float] = None
    drip_line_spacing_m: Optional[float] = None
    irrigation_efficiency: Optional[float] = None
    farmer_name: Optional[str] = None
    farmer_phone: Optional[str] = None
    farmer_chat_id: Optional[str] = None
    language: Optional[str] = None
    quiet_hours_start: Optional[str] = None
    quiet_hours_end: Optional[str] = None
    digest_enabled: Optional[bool] = None
    digest_day: Optional[int] = None
    digest_hour: Optional[int] = None
    escalation_phone: Optional[str] = None
    active: Optional[bool] = None
    paused_until: Optional[datetime] = None

class SensorCreateRequest(BaseModel):
    """Payload for registering a soil probe against a farm block."""
    deveui: str
    name: Optional[str] = None
    model_type: str = 'Dragino-LSE01'
    install_depth_cm: int = 40
    zone: Optional[str] = None
    valve_id: Optional[str] = None
    is_simulated: bool = False

class SensorUpdateRequest(BaseModel):
    """Partial update of a soil probe."""
    name: Optional[str] = None
    model_type: Optional[str] = None
    install_depth_cm: Optional[int] = None
    zone: Optional[str] = None
    valve_id: Optional[str] = None
    is_simulated: Optional[bool] = None
    field_id: Optional[int] = None

class SensorBindRequest(BaseModel):
    """Bind an existing (possibly orphaned) probe to a farm block."""
    field_id: int
    zone: Optional[str] = None

class LoginRequest(BaseModel):
    email: str
    password: str

class AuthTokenResponse(BaseModel):
    access_token: str
    token_type: str = 'bearer'
    user_id: int
    email: str
    role: str
    full_name: str

# Ensure database tables exist at import time
create_db_and_tables()

@asynccontextmanager
async def lifespan(app: FastAPI):
    # Fail fast on insecure production configuration instead of silently shipping
    # the development secret key into a live deployment.
    if settings.is_production:
        if not settings.SECRET_KEY or settings.SECRET_KEY == 'fieldsense-dev-insecure-secret-key-change-me':
            raise RuntimeError('SECRET_KEY must be set to a unique value when ENVIRONMENT=production')
        if not settings.TTN_WEBHOOK_SECRET:
            logger.warning("[SECURITY] TTN_WEBHOOK_SECRET is not set - telemetry ingestion will be rejected")
    logger.info("[STARTUP] Initializing FieldSense database and background services...")
    create_db_and_tables()
    await DailyAdvisoryScheduler.start()
    yield
    logger.info("[SHUTDOWN] Stopping background services...")
    await DailyAdvisoryScheduler.stop()

app = FastAPI(
    title='TerraFlux Platform (FieldSense Engine)',
    description='Autonomous IoT Viticulture & WhatsApp Advisory Platform with Physical Soil Balance & BOM Feasibility Engine',
    version='1.0.0',
    lifespan=lifespan
)

# CORS Policy
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.CORS_ORIGINS,
    allow_credentials=False if '*' in settings.CORS_ORIGINS else True,
    allow_methods=['GET', 'POST', 'OPTIONS'],
    allow_headers=['*']
)

# Request ID and Tracing Middleware
@app.middleware('http')
async def request_id_and_logging_middleware(request: Request, call_next):
    request_id = request.headers.get('X-Request-ID', str(uuid.uuid4()))
    request.state.request_id = request_id
    
    start_time = datetime.now(timezone.utc)
    response: Response = await call_next(request)
    duration_ms = round((datetime.now(timezone.utc) - start_time).total_seconds() * 1000, 2)
    
    response.headers['X-Request-ID'] = request_id
    response.headers['X-Content-Type-Options'] = 'nosniff'
    response.headers['X-Frame-Options'] = 'SAMEORIGIN'
    response.headers['Referrer-Policy'] = 'strict-origin-when-cross-origin'
    logger.info(f"[{request.method}] {request.url.path} -> {response.status_code} ({duration_ms}ms) | req_id={request_id}")
    return response

# ==========================================
# 1. Health, Readiness & Liveness Probes
# ==========================================

@app.get('/healthz')
@app.get('/api/v1/health')
def liveness_check():
    """Liveness probe: verifies the process is running."""
    return {
        'status': 'healthy',
        'service': 'FieldSense Irrigation',
        'environment': settings.ENVIRONMENT,
        'timestamp': datetime.now(timezone.utc).isoformat()
    }

@app.get('/readyz')
def readiness_check(session: Session = Depends(get_session)):
    """Readiness probe: verifies database connectivity and core services."""
    try:
        session.exec(text('SELECT 1')).first()
        return {'status': 'ready', 'database': 'connected', 'timestamp': datetime.now(timezone.utc).isoformat()}
    except Exception as e:
        logger.error(f"[READINESS FAILED] Database connectivity error: {e}")
        raise HTTPException(status_code=status.HTTP_503_SERVICE_UNAVAILABLE, detail='Database unavailable')

# ==========================================
# 1b. Multi-Tenant Authentication & Accounts
# ==========================================

@app.post('/api/v1/auth/register', response_model=AuthTokenResponse)
def register_user(payload: RegisterRequest, request: Request, session: Session = Depends(get_session)):
    """Self-serve registration for farm owners and agronomists."""
    enforce_rate_limit(
        auth_limiter,
        f'register:{client_key(request)}',
        settings.REGISTER_RATE_LIMIT_PER_HOUR,
        3600,
        'Too many registration attempts'
    )
    clean_email = payload.email.strip().lower()
    if not clean_email or '@' not in clean_email:
        raise HTTPException(status_code=400, detail='Valid email address is required')
    password_error = validate_password_strength(payload.password)
    if password_error:
        raise HTTPException(status_code=400, detail=password_error)

    existing = session.exec(select(UserModel).where(UserModel.email == clean_email)).first()
    if existing:
        raise HTTPException(status_code=400, detail='A user with this email is already registered')

    user = UserModel(
        email=clean_email,
        hashed_password=hash_password(payload.password),
        full_name=payload.full_name or 'Grower',
        # Public registration can never create an administrator (privilege escalation guard).
        role='grower',
        is_active=True
    )
    session.add(user)
    session.commit()
    session.refresh(user)

    token = create_access_token(
        user_id=user.id, email=user.email, role=user.role, token_version=user.token_version
    )
    return AuthTokenResponse(
        access_token=token,
        token_type='bearer',
        user_id=user.id,
        email=user.email,
        role=user.role,
        full_name=user.full_name
    )

@app.post('/api/v1/auth/login', response_model=AuthTokenResponse)
def login_user(payload: LoginRequest, request: Request, session: Session = Depends(get_session)):
    """Authenticate with email and password to receive signed JWT-style bearer token."""
    enforce_rate_limit(
        auth_limiter,
        f'login-ip:{client_key(request)}',
        settings.LOGIN_RATE_LIMIT_PER_MINUTE,
        60,
        'Too many login attempts from this address'
    )

    clean_email = payload.email.strip().lower()
    lock_key = f'login-user:{clean_email}'
    locked, remaining = auth_limiter.is_locked(lock_key)
    if locked:
        logger.warning(f"[SECURITY] Login attempt for locked account '{clean_email}'")
        raise HTTPException(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail=f'Too many failed attempts. Account locked for {remaining} seconds.',
            headers={'Retry-After': str(remaining)}
        )

    user = session.exec(select(UserModel).where(UserModel.email == clean_email)).first()
    if not user or not verify_password(payload.password, user.hashed_password):
        # Count the failure for unknown accounts too, so the endpoint cannot be used
        # to enumerate registered email addresses.
        just_locked = auth_limiter.register_failure(
            lock_key, settings.LOGIN_MAX_FAILURES, settings.LOGIN_LOCK_SECONDS
        )
        if just_locked:
            logger.warning(f"[SECURITY] Account '{clean_email}' locked after repeated failed logins")
            raise HTTPException(
                status_code=status.HTTP_429_TOO_MANY_REQUESTS,
                detail=f'Too many failed attempts. Account locked for {settings.LOGIN_LOCK_SECONDS} seconds.',
                headers={'Retry-After': str(settings.LOGIN_LOCK_SECONDS)}
            )
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail='Invalid email or password')
    if not user.is_active:
        raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail='User account is deactivated')

    auth_limiter.reset(lock_key)
    token = create_access_token(
        user_id=user.id, email=user.email, role=user.role, token_version=user.token_version
    )
    return AuthTokenResponse(
        access_token=token,
        token_type='bearer',
        user_id=user.id,
        email=user.email,
        role=user.role,
        full_name=user.full_name
    )

@app.post('/api/v1/auth/logout')
def logout_user(
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Revoke every token issued for the current user (server-side logout)."""
    current_user.token_version = int(current_user.token_version) + 1
    session.add(current_user)
    session.commit()
    logger.info(f"[AUTH] Tokens revoked for user {current_user.id} (logout)")
    return {'status': 'logged_out', 'revoked_tokens': True}

@app.get('/api/v1/auth/me')
def get_current_user_profile(
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Retrieve profile and list of owned farm fields for authenticated user."""
    user_fields = session.exec(select(FieldModel).where(FieldModel.user_id == current_user.id)).all()
    return {
        'id': current_user.id,
        'email': current_user.email,
        'full_name': current_user.full_name,
        'role': current_user.role,
        'is_active': current_user.is_active,
        'created_at': current_user.created_at.isoformat(),
        'fields_count': len(user_fields),
        'fields': [
            {
                'id': f.id,
                'name': f.name,
                'location': f.location_name,
                'crop_key': f.crop_key,
                'area_ha': f.area_ha,
                'farmer_chat_id': f.farmer_chat_id,
                'invite_token': f.invite_token,
                'telegram_deep_link': f"https://t.me/TerraFluxBot?start={f.invite_token}"
            }
            for f in user_fields
        ]
    }

@app.post('/api/v1/admin/users/{user_id}/role')
def set_user_role(
    user_id: int,
    payload: RoleUpdateRequest,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Promote or demote a user account. Administrator-only.

    Replaces the previous behaviour where a self-registering client could simply
    pass {"role": "admin"} and receive administrator privileges.
    """
    if current_user.role != 'admin':
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail='Administrator role required to manage user roles'
        )

    target = session.get(UserModel, user_id)
    if not target:
        raise HTTPException(status_code=404, detail='User not found')

    new_role = payload.role.strip().lower()
    if new_role not in ('admin', 'grower'):
        raise HTTPException(status_code=400, detail="Role must be 'admin' or 'grower'")

    if target.id == current_user.id and new_role != 'admin':
        raise HTTPException(
            status_code=400,
            detail='Cannot demote your own administrator account'
        )

    target.role = new_role
    session.add(target)
    session.commit()
    session.refresh(target)
    logger.info(f"[AUTH] Role for user {target.id} set to '{new_role}' by admin {current_user.id}")
    return {'user_id': target.id, 'email': target.email, 'role': target.role, 'is_active': target.is_active}

# ==========================================
# 2. Core REST Endpoints: Fields & Telemetry
# ==========================================

@app.get('/api/v1/fields')
def list_fields(
    limit: int = Query(default=20, ge=1, le=100),
    offset: int = Query(default=0, ge=0),
    include_inactive: bool = Query(default=False, description='Also list deactivated blocks'),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """List registered fields with pagination, multi-tenant isolation, and soil status.

    Requires authentication. Growers only see the fields they own; administrators see
    all fields. Contact details and Telegram invite tokens are only disclosed to the
    owner of a field (or an administrator).
    """
    statement = select(FieldModel)
    if current_user.role != 'admin':
        statement = statement.where(FieldModel.user_id == current_user.id)
    if not include_inactive:
        statement = statement.where(FieldModel.active == True)  # noqa: E712 (SQLAlchemy column truthiness)
    statement = statement.offset(offset).limit(limit)
    enforce_rate_limit(
        api_limiter, f'api:{current_user.id}', settings.API_RATE_LIMIT_PER_MINUTE, 60,
        'API rate limit exceeded'
    )
    fields = session.exec(statement).all()
    results = []
    
    for f in fields:
        sensor = session.exec(select(SensorModel).where(SensorModel.field_id == f.id)).first()
        status = WaterBalanceEngine.analyze_soil_status(
            current_moisture_pct=sensor.last_moisture_pct if sensor else 22.0,
            crop_key=f.crop_key,
            growth_stage=f.growth_stage,
            soil_key=f.soil_key
        )
        may_see_contact = (current_user.role == 'admin') or (f.user_id == current_user.id)
        results.append({
            'id': f.id,
            'name': f.name,
            'location': f.location_name,
            'latitude': f.latitude,
            'longitude': f.longitude,
            'area_ha': f.area_ha,
            'crop_key': f.crop_key,
            'growth_stage': f.growth_stage,
            'soil_key': f.soil_key,
            'timezone': f.timezone or 'Europe/Berlin',
            'daily_alert_hour': f.daily_alert_hour,
            'application_rate_mm_h': round(f.application_rate_mm_h, 2),
            'farmer_name': f.farmer_name if may_see_contact else None,
            'farmer_phone': f.farmer_phone if may_see_contact else None,
            'farmer_chat_id': f.farmer_chat_id if may_see_contact else None,
            'invite_token': f.invite_token if may_see_contact else None,
            'telegram_deep_link': (f"https://t.me/TerraFluxBot?start={f.invite_token}") if may_see_contact else None,
            'sensor': {
                'id': sensor.id if sensor else None,
                'name': sensor.name if sensor else None,
                'deveui': sensor.deveui if sensor else None,
                'last_moisture_pct': sensor.last_moisture_pct if sensor else 20.0,
                'last_temp_c': sensor.last_temp_c if sensor else 20.0,
                'battery_volts': sensor.battery_volts if sensor else 3.6,
                'last_seen': sensor.last_seen.isoformat() if sensor else None
            } if sensor else None,
            'soil_status': {
                'field_capacity_pct': status.field_capacity_pct,
                'wilting_point_pct': status.wilting_point_pct,
                'saturation_pct': status.saturation_pct,
                'raw_mm': status.raw_mm,
                'taw_mm': status.taw_mm,
                'threshold_moisture_pct': status.threshold_moisture_pct,
                'current_moisture_pct': status.current_moisture_pct,
                'depletion_mm': status.depletion_mm,
                'stress_index': status.stress_index,
                'is_stressed': status.is_stressed
            }
        })
    return results

@app.post('/api/v1/fields/{field_id}/invite-token')
def refresh_field_invite_token(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Regenerate private Telegram onboarding invite token for a farm field."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    if current_user.role != 'admin' and field.user_id != current_user.id:
        raise HTTPException(status_code=403, detail='Permission denied to manage this field')

    import secrets
    field.invite_token = secrets.token_hex(4)
    session.add(field)
    session.commit()
    session.refresh(field)

    return {
        'field_id': field.id,
        'field_name': field.name,
        'invite_token': field.invite_token,
        'telegram_deep_link': f"https://t.me/TerraFluxBot?start={field.invite_token}"
    }

@app.post('/api/v1/fields', status_code=status.HTTP_201_CREATED)
def create_field(
    payload: FieldCreateRequest,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Create a new farm block owned by the caller (administrators may assign an owner)."""
    enforce_rate_limit(
        api_limiter, f'api:{current_user.id}', settings.API_RATE_LIMIT_PER_MINUTE, 60,
        'API rate limit exceeded'
    )
    if not payload.name.strip():
        raise HTTPException(status_code=400, detail='Field name is required')
    validate_agronomy(payload.crop_key, payload.growth_stage, payload.soil_key)

    owner_id = current_user.id
    if payload.user_id is not None:
        if current_user.role != 'admin':
            raise HTTPException(status_code=403, detail='Only administrators may assign another owner')
        owner_id = payload.user_id

    field = FieldModel(
        user_id=owner_id,
        name=payload.name.strip(),
        location_name=payload.location_name or 'Unspecified',
        latitude=payload.latitude if payload.latitude is not None else 48.0833,
        longitude=payload.longitude if payload.longitude is not None else 7.6833,
        area_ha=payload.area_ha,
        crop_key=payload.crop_key,
        growth_stage=payload.growth_stage,
        soil_key=payload.soil_key,
        timezone=payload.timezone,
        daily_alert_hour=max(0, min(23, payload.daily_alert_hour)),
        drip_emitter_spacing_m=payload.drip_emitter_spacing_m,
        drip_emitter_lph=payload.drip_emitter_lph,
        drip_line_spacing_m=payload.drip_line_spacing_m,
        irrigation_efficiency=payload.irrigation_efficiency,
        farmer_name=payload.farmer_name or current_user.full_name,
        farmer_phone=payload.farmer_phone or '',
        farmer_chat_id=payload.farmer_chat_id,
        language=normalize_language(payload.language),
        quiet_hours_start=payload.quiet_hours_start,
        quiet_hours_end=payload.quiet_hours_end,
        digest_enabled=payload.digest_enabled,
        digest_day=max(0, min(6, payload.digest_day)),
        digest_hour=max(0, min(23, payload.digest_hour)),
        escalation_phone=payload.escalation_phone,
        active=True
    )
    session.add(field)
    session.commit()
    session.refresh(field)
    logger.info(f"[FIELD CREATED] '{field.name}' (id={field.id}) by user {current_user.id}")
    return {
        'id': field.id,
        'name': field.name,
        'location': field.location_name,
        'area_ha': field.area_ha,
        'crop_key': field.crop_key,
        'growth_stage': field.growth_stage,
        'soil_key': field.soil_key,
        'timezone': field.timezone,
        'daily_alert_hour': field.daily_alert_hour,
        'invite_token': field.invite_token,
        'telegram_deep_link': f"https://t.me/TerraFluxBot?start={field.invite_token}"
    }

@app.get('/api/v1/fields/{field_id}')
def get_field(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Full detail of one farm block: sensors, per-zone status and configuration."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).all()
    status = WaterBalanceEngine.analyze_soil_status(
        current_moisture_pct=sensors[0].last_moisture_pct if sensors else 22.0,
        crop_key=field.crop_key,
        growth_stage=field.growth_stage,
        soil_key=field.soil_key
    )
    return {
        'id': field.id,
        'name': field.name,
        'location': field.location_name,
        'latitude': field.latitude,
        'longitude': field.longitude,
        'area_ha': field.area_ha,
        'crop_key': field.crop_key,
        'growth_stage': field.growth_stage,
        'soil_key': field.soil_key,
        'timezone': field.timezone,
        'daily_alert_hour': field.daily_alert_hour,
        'active': field.active,
        'paused_until': field.paused_until.isoformat() if field.paused_until else None,
        'application_rate_mm_h': round(field.application_rate_mm_h, 2),
        'gross_application_rate_mm_h': round(field.gross_application_rate_mm_h, 2),
        'irrigation_efficiency': field.irrigation_efficiency,
        'farmer_name': field.farmer_name,
        'farmer_phone': field.farmer_phone,
        'farmer_chat_id': field.farmer_chat_id,
        'invite_token': field.invite_token,
        'telegram_deep_link': f"https://t.me/TerraFluxBot?start={field.invite_token}",
        'soil_status': {
            'field_capacity_pct': status.field_capacity_pct,
            'wilting_point_pct': status.wilting_point_pct,
            'saturation_pct': status.saturation_pct,
            'raw_mm': status.raw_mm,
            'taw_mm': status.taw_mm,
            'threshold_moisture_pct': status.threshold_moisture_pct,
            'current_moisture_pct': status.current_moisture_pct,
            'depletion_mm': status.depletion_mm,
            'stress_index': status.stress_index,
            'is_stressed': status.is_stressed
        },
        'sensors': [_sensor_payload(s) for s in sensors],
        'zones': _zone_payload(field, sensors)
    }

@app.patch('/api/v1/fields/{field_id}')
def update_field(
    field_id: int,
    payload: FieldUpdateRequest,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Update the mutable configuration of a farm block."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    data = payload.model_dump(exclude_unset=True, exclude_none=True)

    # Agronomy must stay consistent before it is written
    validate_agronomy(
        data.get('crop_key', field.crop_key),
        data.get('growth_stage', field.growth_stage),
        data.get('soil_key', field.soil_key)
    )

    if 'daily_alert_hour' in data:
        data['daily_alert_hour'] = max(0, min(23, int(data['daily_alert_hour'])))
    if 'irrigation_efficiency' in data:
        eff = float(data['irrigation_efficiency'])
        if not (0.1 <= eff <= 1.0):
            raise HTTPException(status_code=400, detail='Irrigation efficiency must be between 0.1 and 1.0')
    if 'name' in data:
        data['name'] = str(data['name']).strip()
        if not data['name']:
            raise HTTPException(status_code=400, detail='Field name cannot be empty')

    for key, value in data.items():
        setattr(field, key, value)

    session.add(field)
    session.commit()
    session.refresh(field)
    logger.info(f"[FIELD UPDATED] '{field.name}' (id={field.id}) by user {current_user.id}: {sorted(data.keys())}")
    return {'id': field.id, 'name': field.name, 'updated': sorted(data.keys()), 'active': field.active}

@app.delete('/api/v1/fields/{field_id}')
def deactivate_field(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Deactivate a farm block. The block keeps its history but stops dispatching."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    field.active = False
    session.add(field)
    session.commit()
    logger.info(f"[FIELD DEACTIVATED] '{field.name}' (id={field.id}) by user {current_user.id}")
    return {'id': field.id, 'name': field.name, 'active': False}

@app.get('/api/v1/fields/{field_id}/sensors')
def list_field_sensors(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """List the probes registered for a farm block."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)
    sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).all()
    return {
        'field_id': field.id,
        'sensors': [_sensor_payload(s) for s in sensors],
        'zones': _zone_payload(field, sensors)
    }

@app.post('/api/v1/fields/{field_id}/sensors', status_code=status.HTTP_201_CREATED)
def register_field_sensor(
    field_id: int,
    payload: SensorCreateRequest,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Register a soil probe (DevEUI) against a farm block."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    deveui = payload.deveui.strip()
    if not deveui:
        raise HTTPException(status_code=400, detail='DevEUI is required')
    existing = session.exec(select(SensorModel).where(SensorModel.deveui == deveui)).first()
    if existing:
        raise HTTPException(
            status_code=409,
            detail=f"Probe '{deveui}' is already registered"
            + (f" on field {existing.field_id}" if existing.field_id else ' without a field')
        )

    sensor = SensorModel(
        deveui=deveui,
        name=payload.name or f'Probe-{deveui[-4:]}',
        field_id=field.id,
        model_type=payload.model_type,
        install_depth_cm=payload.install_depth_cm,
        zone=payload.zone.strip() if payload.zone else None,
        valve_id=payload.valve_id.strip() if payload.valve_id else None,
        is_simulated=payload.is_simulated
    )
    session.add(sensor)
    session.commit()
    session.refresh(sensor)
    logger.info(f"[SENSOR REGISTERED] {deveui} -> field '{field.name}' (zone={sensor.zone})")
    return _sensor_payload(sensor)

@app.patch('/api/v1/sensors/{sensor_id}')
def update_sensor(
    sensor_id: int,
    payload: SensorUpdateRequest,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Update a probe (name, depth, zone, or move it to another block)."""
    sensor = session.get(SensorModel, sensor_id)
    if not sensor:
        raise HTTPException(status_code=404, detail='Sensor not found')
    if sensor.field_id:
        field = session.get(FieldModel, sensor.field_id)
        if field:
            require_field_access(field, current_user)
    elif current_user.role != 'admin':
        raise HTTPException(status_code=403, detail='Only administrators may edit an unbound probe')

    data = payload.model_dump(exclude_unset=True, exclude_none=True)
    if 'field_id' in data:
        target = session.get(FieldModel, data['field_id'])
        if not target:
            raise HTTPException(status_code=404, detail='Target field not found')
        require_field_access(target, current_user)
    if 'zone' in data and data['zone']:
        data['zone'] = str(data['zone']).strip()
    if 'valve_id' in data and data['valve_id']:
        data['valve_id'] = str(data['valve_id']).strip()

    for key, value in data.items():
        setattr(sensor, key, value)

    session.add(sensor)
    session.commit()
    session.refresh(sensor)
    return _sensor_payload(sensor)

@app.post('/api/v1/sensors/{sensor_id}/bind')
def bind_sensor(
    sensor_id: int,
    payload: SensorBindRequest,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Bind an existing probe (for example one auto-registered by a TTN uplink) to a block."""
    sensor = session.get(SensorModel, sensor_id)
    if not sensor:
        raise HTTPException(status_code=404, detail='Sensor not found')
    if sensor.field_id:
        current_field = session.get(FieldModel, sensor.field_id)
        if current_field:
            require_field_access(current_field, current_user)

    target = session.get(FieldModel, payload.field_id)
    if not target:
        raise HTTPException(status_code=404, detail='Target field not found')
    require_field_access(target, current_user)

    sensor.field_id = target.id
    if payload.zone:
        sensor.zone = payload.zone.strip()
    session.add(sensor)
    session.commit()
    session.refresh(sensor)
    logger.info(f"[SENSOR BOUND] {sensor.deveui} -> field '{target.name}' (zone={sensor.zone})")
    return _sensor_payload(sensor)

@app.delete('/api/v1/sensors/{sensor_id}')
def delete_sensor(
    sensor_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Remove a probe registration together with its telemetry history."""
    sensor = session.get(SensorModel, sensor_id)
    if not sensor:
        raise HTTPException(status_code=404, detail='Sensor not found')
    if sensor.field_id:
        field = session.get(FieldModel, sensor.field_id)
        if field:
            require_field_access(field, current_user)
    elif current_user.role != 'admin':
        raise HTTPException(status_code=403, detail='Only administrators may delete an unbound probe')

    for reading in session.exec(
        select(TelemetryReadingModel).where(TelemetryReadingModel.sensor_id == sensor.id)
    ).all():
        session.delete(reading)
    deveui = sensor.deveui
    session.delete(sensor)
    session.commit()
    logger.info(f"[SENSOR REMOVED] {deveui}")
    return {'status': 'removed', 'deveui': deveui}

@app.get('/api/v1/fields/{field_id}/advisories')
def list_field_advisories(
    field_id: int,
    limit: int = Query(default=50, ge=1, le=500),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Advisory history of a farm block (newest first)."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    alerts = session.exec(
        select(AdvisoryAlertModel)
        .where(AdvisoryAlertModel.field_id == field_id)
        .order_by(AdvisoryAlertModel.timestamp.desc())
        .limit(limit)
    ).all()

    return {
        'field_id': field_id,
        'advisories': [
            {
                'id': a.id,
                'timestamp': a.timestamp.isoformat(),
                'status': a.status,
                'soil_moisture_pct': a.soil_moisture_pct,
                'threshold_pct': a.threshold_pct,
                'forecast_et0_mm': a.forecast_et0_mm,
                'forecast_rain_mm': a.forecast_rain_mm,
                'net_depth_mm': a.recommended_depth_mm,
                'gross_depth_mm': a.recommended_gross_mm,
                'duration_minutes': a.recommended_duration_min,
                'start_time': a.recommended_start_time,
                'message': a.message_text
            }
            for a in alerts
        ]
    }

@app.get('/api/v1/fields/{field_id}/irrigations')
def list_field_irrigations(
    field_id: int,
    limit: int = Query(default=50, ge=1, le=500),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Irrigation event history of a farm block (newest first)."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    events = session.exec(
        select(IrrigationEventModel)
        .where(IrrigationEventModel.field_id == field_id)
        .order_by(IrrigationEventModel.timestamp.desc())
        .limit(limit)
    ).all()

    return {
        'field_id': field_id,
        'irrigations': [
            {
                'id': e.id,
                'timestamp': e.timestamp.isoformat(),
                'net_depth_mm': e.depth_applied_mm,
                'duration_minutes': e.duration_minutes,
                'source': e.source,
                'notes': e.notes
            }
            for e in events
        ],
        'total_net_depth_mm': round(sum(e.depth_applied_mm for e in events), 1),
        'total_minutes': sum(e.duration_minutes for e in events)
    }

@app.get('/api/v1/fields/{field_id}/dispatches')
def list_field_dispatches(
    field_id: int,
    limit: int = Query(default=50, ge=1, le=500),
    status: Optional[str] = Query(default=None, description='Filter by status (sent/simulated/failed/skipped)'),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Delivery audit trail of a farm block (newest first)."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    statement = select(DispatchLogModel).where(DispatchLogModel.field_id == field_id)
    if status:
        statement = statement.where(DispatchLogModel.status == status)
    entries = session.exec(
        statement.order_by(DispatchLogModel.created_at.desc()).limit(limit)
    ).all()

    return {
        'field_id': field_id,
        'dispatches': [
            {
                'id': d.id,
                'created_at': d.created_at.isoformat(),
                'channel': d.channel,
                'target': d.target,
                'status': d.status,
                'attempts': d.attempts,
                'detail': d.detail,
                'alert_id': d.alert_id
            }
            for d in entries
        ],
        'failed': sum(1 for d in entries if d.status in ('error', 'exception', 'failed'))
    }

@app.get('/api/v1/fields/{field_id}/history')
def get_field_history(
    field_id: int,
    limit: int = Query(default=25, ge=1, le=200),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Combined operational history of a farm block."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    alerts = session.exec(
        select(AdvisoryAlertModel)
        .where(AdvisoryAlertModel.field_id == field_id)
        .order_by(AdvisoryAlertModel.timestamp.desc())
        .limit(limit)
    ).all()
    events = session.exec(
        select(IrrigationEventModel)
        .where(IrrigationEventModel.field_id == field_id)
        .order_by(IrrigationEventModel.timestamp.desc())
        .limit(limit)
    ).all()
    dispatches = session.exec(
        select(DispatchLogModel)
        .where(DispatchLogModel.field_id == field_id)
        .order_by(DispatchLogModel.created_at.desc())
        .limit(limit)
    ).all()

    return {
        'field_id': field_id,
        'field_name': field.name,
        'summary': {
            'advisories': len(alerts),
            'irrigations': len(events),
            'dispatches': len(dispatches),
            'failed_dispatches': sum(1 for d in dispatches if d.status in ('error', 'exception', 'failed')),
            'applied_net_depth_mm': round(sum(e.depth_applied_mm for e in events), 1),
            'applied_minutes': sum(e.duration_minutes for e in events),
            'last_advisory_at': alerts[0].timestamp.isoformat() if alerts else None,
            'last_irrigation_at': events[0].timestamp.isoformat() if events else None
        },
        'advisories': [
            {
                'id': a.id,
                'timestamp': a.timestamp.isoformat(),
                'status': a.status,
                'net_depth_mm': a.recommended_depth_mm,
                'gross_depth_mm': a.recommended_gross_mm,
                'duration_minutes': a.recommended_duration_min,
                'start_time': a.recommended_start_time
            }
            for a in alerts
        ],
        'irrigations': [
            {
                'id': e.id,
                'timestamp': e.timestamp.isoformat(),
                'net_depth_mm': e.depth_applied_mm,
                'duration_minutes': e.duration_minutes,
                'source': e.source,
                'notes': e.notes
            }
            for e in events
        ],
        'dispatches': [
            {
                'id': d.id,
                'created_at': d.created_at.isoformat(),
                'channel': d.channel,
                'status': d.status,
                'attempts': d.attempts,
                'detail': d.detail
            }
            for d in dispatches
        ]
    }

@app.get('/api/v1/fields/{field_id}/actuation')
def get_field_actuation(
    field_id: int,
    limit: int = Query(default=25, ge=1, le=200),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Valve plans and valve commands of a farm block (audit trail)."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    events = list_actuation_events(session, field_id, limit)
    return {
        'field_id': field_id,
        'controller': 'http' if settings.VALVE_CONTROLLER_URL else 'simulated',
        'auto_actuation_enabled': settings.AUTO_ACTUATION_ENABLED,
        'events': [
            {
                'id': e.id,
                'created_at': e.created_at.isoformat(),
                'zone': e.zone,
                'valve_id': e.valve_id,
                'status': e.status,
                'start_time': e.start_time,
                'duration_minutes': e.duration_minutes,
                'gross_depth_mm': e.gross_depth_mm,
                'net_depth_mm': e.net_depth_mm,
                'detail': e.detail
            }
            for e in events
        ]
    }

@app.post('/api/v1/fields/{field_id}/actuation/plan')
def create_field_actuation_plan(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Compute (and store) the valve plan for the current advisory. No hardware is touched."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).all()
    critical = AgronomicAdvisor.select_critical_zone(field, sensors)
    if critical:
        _, zone, valve_id, moisture, stale_hours = critical
    else:
        zone = valve_id = None
        moisture, stale_hours = 20.0, 0.0

    weather = WeatherService.get_forecast(field.latitude, field.longitude)
    decision = AgronomicAdvisor.evaluate_field(
        field, moisture, weather, hours_since_reading=stale_hours, zone=zone, valve_id=valve_id
    )
    plan = build_plan(field, decision)
    if plan is None:
        return {'field_id': field_id, 'plan': None, 'reason': decision.reason, 'action_required': False}

    event = record_actuation_event(session, plan, status='planned', detail=decision.reason)
    return {
        'field_id': field_id,
        'action_required': True,
        'plan': plan.to_dict(),
        'event_id': event.id,
        'reason': decision.reason
    }

@app.post('/api/v1/fields/{field_id}/actuation/execute')
def execute_field_actuation(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Hand the valve plan to the controller (simulated unless a gateway is configured)."""
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).all()
    critical = AgronomicAdvisor.select_critical_zone(field, sensors)
    if critical:
        _, zone, valve_id, moisture, stale_hours = critical
    else:
        zone = valve_id = None
        moisture, stale_hours = 20.0, 0.0

    weather = WeatherService.get_forecast(field.latitude, field.longitude)
    decision = AgronomicAdvisor.evaluate_field(
        field, moisture, weather, hours_since_reading=stale_hours, zone=zone, valve_id=valve_id
    )
    plan = build_plan(field, decision)
    if plan is None:
        raise HTTPException(status_code=409, detail='No irrigation required at the moment')

    result = execute_actuation_plan(session, plan)
    return {'field_id': field_id, **result}

@app.get('/api/v1/fields/{field_id}/telemetry')
def get_field_telemetry(
    field_id: int,
    limit: int = Query(default=100, ge=1, le=500),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Retrieve historical telemetry readings for a field's probe (owner or admin only)."""
    enforce_rate_limit(
        api_limiter, f'api:{current_user.id}', settings.API_RATE_LIMIT_PER_MINUTE, 60,
        'API rate limit exceeded'
    )
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    sensor = session.exec(select(SensorModel).where(SensorModel.field_id == field_id)).first()
    if not sensor:
        return {'readings': []}
    
    readings = session.exec(
        select(TelemetryReadingModel)
        .where(TelemetryReadingModel.sensor_id == sensor.id)
        .order_by(TelemetryReadingModel.timestamp.desc())
        .limit(limit)
    ).all()

    return {
        'sensor': {
            'id': sensor.id,
            'name': sensor.name,
            'deveui': sensor.deveui,
            'battery_volts': sensor.battery_volts
        },
        'readings': [
            {
                'timestamp': r.timestamp.isoformat(),
                'soil_moisture_pct': r.soil_moisture_pct,
                'soil_temp_c': r.soil_temp_c,
                'battery_volts': r.battery_volts,
                'rssi': r.rssi
            }
            for r in reversed(readings)
        ]
    }

# ==========================================
# 3. IoT Ingestion: TTN / LoRaWAN Webhook
# ==========================================

@app.post('/api/v1/telemetry/ttn')
def receive_ttn_uplink(
    payload: Dict[str, Any],
    request: Request,
    x_ttn_webhook_secret: Optional[str] = Header(default=None, alias='X-TTN-Webhook-Secret'),
    authorization: Optional[str] = Header(default=None),
    session: Session = Depends(get_session)
):
    """Production webhook for The Things Network (TTN v3) / ChirpStack uplinks with authentication."""
    enforce_rate_limit(
        webhook_limiter,
        f'ttn:{client_key(request)}',
        settings.WEBHOOK_RATE_LIMIT_PER_MINUTE,
        60,
        'Telemetry ingestion rate limit exceeded'
    )
    # Validate secret token if configured
    if settings.TTN_WEBHOOK_SECRET:
        token = x_ttn_webhook_secret or (authorization.replace('Bearer ', '') if authorization else None)
        if token != settings.TTN_WEBHOOK_SECRET:
            logger.warning("[SECURITY] Rejected unauthorized TTN uplink payload")
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail='Invalid TTN webhook token')
    elif settings.is_production:
        # Fail closed: an open ingestion endpoint in production lets anyone inject fake
        # soil readings and drive the agronomic model.
        logger.error("[SECURITY] TTN_WEBHOOK_SECRET is not configured in production")
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail='Telemetry ingestion is not configured'
        )

    decoded = LoRaWANDecoder.decode_ttn_webhook(payload)
    deveui = decoded['deveui']

    sensor = session.exec(select(SensorModel).where(SensorModel.deveui == deveui)).first()
    previous_60cm = sensor.moisture_60cm_pct if sensor else None

    # Effective root-zone moisture: weighted 25/50/25 over the reported depths.
    effective_moisture, moisture_source = WaterBalanceEngine.compute_root_zone_moisture(
        moisture_15cm_pct=decoded.get('moisture_15cm_pct'),
        moisture_30cm_pct=decoded.get('moisture_30cm_pct'),
        moisture_60cm_pct=decoded.get('moisture_60cm_pct'),
        fallback_pct=decoded['soil_moisture_pct']
    )
    deep_percolation = WaterBalanceEngine.detect_deep_percolation(
        previous_60cm_pct=previous_60cm,
        current_60cm_pct=decoded.get('moisture_60cm_pct')
    )

    if not sensor:
        sensor = SensorModel(
            deveui=deveui,
            name=f'Probe-{deveui[-4:]}',
            battery_volts=decoded['battery_volts'],
            last_moisture_pct=effective_moisture,
            last_temp_c=decoded['soil_temp_c'],
            last_seen=datetime.now(timezone.utc)
        )
        session.add(sensor)
        session.commit()
        session.refresh(sensor)
        logger.info(f"[AUTO-REGISTER] Registered new probe: {deveui}")
    else:
        sensor.battery_volts = decoded['battery_volts']
        sensor.last_moisture_pct = effective_moisture
        sensor.last_temp_c = decoded['soil_temp_c']
        sensor.last_seen = datetime.now(timezone.utc)
        session.add(sensor)

    sensor.moisture_15cm_pct = decoded.get('moisture_15cm_pct', sensor.moisture_15cm_pct)
    sensor.moisture_30cm_pct = decoded.get('moisture_30cm_pct', sensor.moisture_30cm_pct)
    sensor.moisture_60cm_pct = decoded.get('moisture_60cm_pct', sensor.moisture_60cm_pct)
    sensor.soil_ec_us_cm = decoded.get('soil_ec_us_cm', sensor.soil_ec_us_cm)
    sensor.deep_percolation = deep_percolation
    session.add(sensor)

    reading = TelemetryReadingModel(
        sensor_id=sensor.id,
        timestamp=datetime.now(timezone.utc),
        soil_moisture_pct=effective_moisture,
        soil_temp_c=decoded['soil_temp_c'],
        battery_volts=decoded['battery_volts'],
        rssi=decoded.get('rssi'),
        snr=decoded.get('snr'),
        moisture_15cm_pct=decoded.get('moisture_15cm_pct'),
        moisture_30cm_pct=decoded.get('moisture_30cm_pct'),
        moisture_60cm_pct=decoded.get('moisture_60cm_pct'),
        soil_ec_us_cm=decoded.get('soil_ec_us_cm'),
        deep_percolation=deep_percolation
    )
    session.add(reading)
    session.commit()

    logger.info(
        f"[TELEMETRY] {deveui} | VWC(root zone) {effective_moisture:.1f}% ({moisture_source})"
        + (" | deep percolation detected" if deep_percolation else "")
    )

    return {
        'status': 'processed',
        'sensor_id': sensor.id,
        'moisture': effective_moisture,
        'moisture_source': moisture_source,
        'deep_percolation': deep_percolation
    }

# ==========================================
# 4. Agronomic Advisory & WhatsApp Dispatch
# ==========================================

@app.post('/api/v1/advisory/evaluate/{field_id}')
def evaluate_field_advisory(
    field_id: int,
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Run FAO-56 decision engine on live field sensor and weather forecast (owner or admin only)."""
    enforce_rate_limit(
        api_limiter, f'api:{current_user.id}', settings.API_RATE_LIMIT_PER_MINUTE, 60,
        'API rate limit exceeded'
    )
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    sensors = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).all()
    sensor = sensors[0] if sensors else None
    current_moisture = sensor.last_moisture_pct if sensor else 20.0
    stale_hours = hours_since_reading(sensor)
    zone = None
    valve_id = None

    # Zone level advisory: the driest / most stressed zone decides the recommendation.
    critical = AgronomicAdvisor.select_critical_zone(field, sensors)
    if critical:
        _, zone, valve_id, current_moisture, stale_hours = critical

    weather = WeatherService.get_forecast(field.latitude, field.longitude)
    decision = AgronomicAdvisor.evaluate_field(
        field, current_moisture, weather,
        hours_since_reading=stale_hours,
        zone=zone,
        valve_id=valve_id
    )
    message_text = WhatsAppFormatter.format_daily_advisory(field, decision, weather)

    alert = AdvisoryAlertModel(
        field_id=field.id,
        timestamp=datetime.now(timezone.utc),
        soil_moisture_pct=current_moisture,
        threshold_pct=decision.soil_status.threshold_moisture_pct,
        forecast_et0_mm=weather.et0_tomorrow_mm,
        forecast_rain_mm=weather.rain_next_48h_mm,
        recommended_depth_mm=decision.target_depth_mm,
        recommended_gross_mm=decision.gross_depth_mm,
        recommended_duration_min=decision.duration_minutes,
        recommended_start_time=decision.optimal_start_time,
        zone=decision.zone,
        urgency=decision.urgency,
        status='PENDING' if decision.should_irrigate else 'AUTO_LOGGED',
        message_text=message_text
    )
    session.add(alert)
    session.commit()
    session.refresh(alert)

    # Deliver through every configured channel; every attempt is persisted.
    dispatch_result = AdvisoryDispatcher.dispatch(
        session=session, field=field, alert=alert, decision=decision, weather=weather
    )

    return {
        'field_id': field.id,
        'field_name': field.name,
        'decision': {
            'should_irrigate': decision.should_irrigate,
            'urgency': decision.urgency,
            'target_depth_mm': decision.target_depth_mm,
            'gross_depth_mm': decision.gross_depth_mm,
            'duration_minutes': decision.duration_minutes,
            'start_time': decision.optimal_start_time,
            'zone': decision.zone,
            'valve_id': decision.valve_id,
            'reason': decision.reason
        },
        'zones': _zone_payload(field, sensors),
        'weather': {
            'et0_today_mm': weather.et0_today_mm,
            'et0_tomorrow_mm': weather.et0_tomorrow_mm,
            'rain_48h_mm': weather.rain_next_48h_mm,
            'max_temp_c': weather.max_temp_tomorrow_c
        },
        'whatsapp_message': message_text,
        'dispatch': dispatch_result.to_dict(),
        'alert_id': alert.id
    }

# ==========================================
# 5. WhatsApp Inbound Webhook (Twilio / Meta)
# ==========================================

@app.post('/api/v1/whatsapp/webhook')
async def handle_whatsapp_webhook(
    request: Request,
    x_twilio_signature: Optional[str] = Header(default=None, alias='X-Twilio-Signature'),
    session: Session = Depends(get_session)
):
    """Production webhook for inbound Twilio / Meta WhatsApp replies with cryptographic signature check."""
    enforce_rate_limit(
        webhook_limiter,
        f'whatsapp:{client_key(request)}',
        settings.WEBHOOK_RATE_LIMIT_PER_MINUTE,
        60,
        'Webhook rate limit exceeded'
    )
    content_type = request.headers.get('content-type', '')
    from_number = '+4915123456789'
    body = 'STATUS'
    params_dict = {}

    if 'application/json' in content_type:
        try:
            json_data = await request.json()
            from_number = json_data.get('From', json_data.get('from', from_number))
            body = json_data.get('Body', json_data.get('body', json_data.get('message', body)))
            params_dict = json_data
        except Exception:
            pass
    else:
        try:
            form_data = await request.form()
            from_number = form_data.get('From', from_number)
            body = form_data.get('Body', body)
            params_dict = dict(form_data)
        except Exception:
            pass

    # Validate Twilio signature if enabled (always enforced in production)
    if settings.ENFORCE_WEBHOOK_AUTH or settings.is_production:
        if not settings.TWILIO_AUTH_TOKEN:
            logger.error("[SECURITY] TWILIO_AUTH_TOKEN is not configured while webhook auth is enforced")
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail='Webhook authentication is not configured'
            )
        is_valid = TwilioWhatsAppClient.validate_signature(str(request.url), params_dict, x_twilio_signature)
        if not is_valid:
            logger.warning("[SECURITY] Rejected invalid Twilio webhook signature")
            raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail='Invalid Twilio signature')

    from_number = str(from_number).replace('whatsapp:', '').strip()
    reply_text = WhatsAppCommandHandler.handle_incoming_message(session, from_number, str(body))

    accept = request.headers.get('accept', '')
    if 'xml' in accept:
        xml_resp = f'<?xml version="1.0" encoding="UTF-8"?><Response><Message>{reply_text}</Message></Response>'
        return HTMLResponse(content=xml_resp, media_type='application/xml')

    return {'reply': reply_text, 'to': from_number}

# ==========================================
# 5b. Telegram Inbound Webhook & Callback Engine
# ==========================================

@app.post('/api/v1/telegram/webhook')
async def handle_telegram_webhook(
    request: Request,
    x_telegram_bot_api_secret_token: Optional[str] = Header(default=None, alias='X-Telegram-Bot-Api-Secret-Token'),
    session: Session = Depends(get_session)
):
    """Production webhook for inbound Telegram Bot updates and interactive inline button callbacks."""
    enforce_rate_limit(
        webhook_limiter,
        f'telegram:{client_key(request)}',
        settings.WEBHOOK_RATE_LIMIT_PER_MINUTE,
        60,
        'Webhook rate limit exceeded'
    )
    if settings.ENFORCE_WEBHOOK_AUTH or settings.is_production:
        if not settings.TELEGRAM_WEBHOOK_SECRET:
            logger.error("[SECURITY] TELEGRAM_WEBHOOK_SECRET is not configured while webhook auth is enforced")
            raise HTTPException(
                status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
                detail='Telegram webhook authentication is not configured'
            )
        if x_telegram_bot_api_secret_token != settings.TELEGRAM_WEBHOOK_SECRET:
            logger.warning("[SECURITY] Rejected unauthorized Telegram webhook secret token")
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail='Invalid secret token')

    try:
        update_data = await request.json()
    except Exception:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail='Invalid JSON payload')

    result = WhatsAppCommandHandler.handle_telegram_update(session, update_data)
    return result

# ==========================================
# 6. Interactive Simulation & Scenarios
# ==========================================

@app.post('/api/v1/simulation/run')
def run_simulation(
    field_id: int = Query(default=1),
    days: int = Query(default=14),
    scenario: str = Query(default='drought'),
    initial_moisture: float = Query(default=24.0),
    persist: bool = Query(
        default=False,
        description='Persist the simulated timeline into the database. Off by default so a '
                    'playground run can never overwrite real probe readings.'
    ),
    current_user: UserModel = Depends(get_current_user),
    session: Session = Depends(get_session)
):
    """Simulates physical soil drying curve and generates telemetry timeline (owner or admin only)."""
    enforce_rate_limit(
        api_limiter, f'api:{current_user.id}', settings.API_RATE_LIMIT_PER_MINUTE, 60,
        'API rate limit exceeded'
    )
    field = session.get(FieldModel, field_id)
    if not field:
        raise HTTPException(status_code=404, detail='Field not found')
    require_field_access(field, current_user)

    sensor = session.exec(select(SensorModel).where(SensorModel.field_id == field.id)).first()
    simulator = SoilSensorSimulator(field, initial_moisture_pct=initial_moisture)
    
    sim_timeline = simulator.run_scenario(days=days, scenario_type=scenario)

    if persist and sensor and sim_timeline:
        last_step = sim_timeline[-1]
        sensor.last_moisture_pct = last_step.soil_moisture_pct
        sensor.last_temp_c = last_step.soil_temp_c
        sensor.battery_volts = last_step.battery_volts
        sensor.last_seen = datetime.now(timezone.utc)
        session.add(sensor)
        
        for item in sim_timeline[::3]:
            rec = TelemetryReadingModel(
                sensor_id=sensor.id,
                timestamp=item.timestamp,
                soil_moisture_pct=item.soil_moisture_pct,
                soil_temp_c=item.soil_temp_c,
                battery_volts=item.battery_volts,
                source='simulation'
            )
            session.add(rec)
        session.commit()

    soil_status = WaterBalanceEngine.analyze_soil_status(
        current_moisture_pct=sim_timeline[-1].soil_moisture_pct,
        crop_key=field.crop_key,
        growth_stage=field.growth_stage,
        soil_key=field.soil_key
    )

    return {
        'field_id': field.id,
        'scenario': scenario,
        'days': days,
        'final_status': {
            'moisture_pct': sim_timeline[-1].soil_moisture_pct,
            'threshold_pct': soil_status.threshold_moisture_pct,
            'is_stressed': soil_status.is_stressed,
            'stress_index': soil_status.stress_index
        },
        'timeline': [
            {
                'timestamp': s.timestamp.isoformat(),
                'soil_moisture_pct': s.soil_moisture_pct,
                'soil_temp_c': s.soil_temp_c,
                'battery_volts': s.battery_volts,
                'etc_mm': s.etc_mm,
                'rain_mm': s.rain_mm,
                'drainage_mm': s.drainage_mm,
                'stress_index': s.stress_index
            }
            for s in sim_timeline
        ]
    }

# ==========================================
# 7. Hardware BOM & Farm Economics Calculator
# ==========================================

@app.post('/api/v1/economics/evaluate')
def evaluate_bom_economics(
    farm_area_ha: float = Query(default=20.0),
    probes_count: int = Query(default=3),
    water_tariff_eur_m3: float = Query(default=1.20),
    crop_type: str = Query(default='grapevine'),
    include_gateway: bool = Query(default=False),
    hardware_tier: str = Query(default='diy')
):
    """Calculates BOM cost breakdown, farmer water bill savings, and payback period."""
    report = BOMEconomicsModel.evaluate_economics(
        farm_area_ha=farm_area_ha,
        probes_count=probes_count,
        water_tariff_eur_m3=water_tariff_eur_m3,
        crop_type=crop_type,
        include_gateway=include_gateway,
        hardware_tier=hardware_tier
    )
    return report

# ==========================================
# 8. Dashboard Web UI
# ==========================================

@app.get('/', response_class=HTMLResponse)
def index_page():
    html_path = pathlib.Path(__file__).resolve().parent.parent / 'ui' / 'templates' / 'index.html'
    if not html_path.exists():
        html_path = pathlib.Path('src/ui/templates/index.html')
    if html_path.exists():
        return HTMLResponse(
            content=html_path.read_text(encoding='utf-8'),
            headers={'Cache-Control': 'no-store, max-age=0'}
        )
    return HTMLResponse(content='<h2>FieldSense Irrigation Backend Running. UI template loading...</h2>')

if __name__ == '__main__':
    import uvicorn
    uvicorn.run('src.api.main:app', host='0.0.0.0', port=8000, reload=True)
