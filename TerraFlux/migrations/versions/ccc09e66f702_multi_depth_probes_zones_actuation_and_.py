"""multi-depth probes, zones, actuation and notification settings

Revision ID: ccc09e66f702
Revises: 4e1fb54be86f
Create Date: 2026-11-10 22:18:05.519037+00:00

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa
import sqlmodel

revision: str = 'ccc09e66f702'
down_revision: Union[str, None] = '4e1fb54be86f'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None

String = sqlmodel.sql.sqltypes.AutoString()


def _existing_columns(table: str) -> set:
    inspector = sa.inspect(op.get_bind())
    return {col['name'] for col in inspector.get_columns(table)}


def _existing_tables() -> set:
    inspector = sa.inspect(op.get_bind())
    return set(inspector.get_table_names())


def _existing_indexes(table: str) -> set:
    inspector = sa.inspect(op.get_bind())
    return {idx['name'] for idx in inspector.get_indexes(table)}


def _add_columns(table: str, columns: list) -> None:
    """Add the missing columns of a table (idempotent).

    The application applies backwards compatible column backfills on startup, so a
    database may already carry some of these columns.
    """
    existing = _existing_columns(table)
    missing = [column for column in columns if column.name not in existing]
    if not missing:
        return
    with op.batch_alter_table(table, schema=None) as batch_op:
        for column in missing:
            batch_op.add_column(column)


def upgrade() -> None:
    """Multi-depth profile, irrigation zones, actuation audit and notification settings."""

    # --- new table: actuation audit trail ---
    if 'actuation_events' not in _existing_tables():
        op.create_table('actuation_events',
            sa.Column('id', sa.Integer(), nullable=False),
            sa.Column('field_id', sa.Integer(), nullable=True),
            sa.Column('alert_id', sa.Integer(), nullable=True),
            sa.Column('zone', String, nullable=True),
            sa.Column('valve_id', String, nullable=True),
            sa.Column('status', String, nullable=False),
            sa.Column('start_time', String, nullable=True),
            sa.Column('duration_minutes', sa.Integer(), nullable=False),
            sa.Column('gross_depth_mm', sa.Float(), nullable=False),
            sa.Column('net_depth_mm', sa.Float(), nullable=False),
            sa.Column('detail', String, nullable=True),
            sa.Column('created_at', sa.DateTime(), nullable=False),
            sa.ForeignKeyConstraint(['alert_id'], ['advisory_alerts.id'], ),
            sa.ForeignKeyConstraint(['field_id'], ['fields.id'], ),
            sa.PrimaryKeyConstraint('id')
        )
        with op.batch_alter_table('actuation_events', schema=None) as batch_op:
            batch_op.create_index(batch_op.f('ix_actuation_events_alert_id'), ['alert_id'], unique=False)
            batch_op.create_index(batch_op.f('ix_actuation_events_created_at'), ['created_at'], unique=False)
            batch_op.create_index(batch_op.f('ix_actuation_events_field_id'), ['field_id'], unique=False)
            batch_op.create_index(batch_op.f('ix_actuation_events_status'), ['status'], unique=False)
            batch_op.create_index(batch_op.f('ix_actuation_events_zone'), ['zone'], unique=False)

    # --- fields: language, quiet hours, weekly digest, escalation contact ---
    _add_columns('fields', [
        sa.Column('language', String, nullable=False, server_default='en'),
        sa.Column('quiet_hours_start', String, nullable=True),
        sa.Column('quiet_hours_end', String, nullable=True),
        sa.Column('digest_enabled', sa.Boolean(), nullable=False, server_default=sa.text('0')),
        sa.Column('digest_day', sa.Integer(), nullable=False, server_default=sa.text('0')),
        sa.Column('digest_hour', sa.Integer(), nullable=False, server_default=sa.text('8')),
        sa.Column('escalation_phone', String, nullable=True),
    ])

    # --- sensors: valve reference and the multi-depth profile ---
    _add_columns('sensors', [
        sa.Column('valve_id', String, nullable=True),
        sa.Column('moisture_15cm_pct', sa.Float(), nullable=True),
        sa.Column('moisture_30cm_pct', sa.Float(), nullable=True),
        sa.Column('moisture_60cm_pct', sa.Float(), nullable=True),
        sa.Column('soil_ec_us_cm', sa.Float(), nullable=True),
        sa.Column('deep_percolation', sa.Boolean(), nullable=False, server_default=sa.text('0')),
    ])

    # --- telemetry readings: the multi-depth profile ---
    _add_columns('telemetry_readings', [
        sa.Column('moisture_15cm_pct', sa.Float(), nullable=True),
        sa.Column('moisture_30cm_pct', sa.Float(), nullable=True),
        sa.Column('moisture_60cm_pct', sa.Float(), nullable=True),
        sa.Column('soil_ec_us_cm', sa.Float(), nullable=True),
        sa.Column('deep_percolation', sa.Boolean(), nullable=False, server_default=sa.text('0')),
    ])

    # --- irrigation events and advisories know their zone ---
    _add_columns('irrigation_events', [
        sa.Column('zone', String, nullable=True),
    ])
    if 'zone' in _existing_columns('irrigation_events'):
        op.create_index('ix_irrigation_events_zone', 'irrigation_events', ['zone'], unique=False)

    _add_columns('advisory_alerts', [
        sa.Column('zone', String, nullable=True),
        sa.Column('urgency', String, nullable=False, server_default='NONE'),
    ])
    if 'zone' in _existing_columns('advisory_alerts'):
        op.create_index('ix_advisory_alerts_zone', 'advisory_alerts', ['zone'], unique=False)
    if 'urgency' in _existing_columns('advisory_alerts'):
        op.create_index('ix_advisory_alerts_urgency', 'advisory_alerts', ['urgency'], unique=False)


def downgrade() -> None:
    """Remove the P2 columns and the actuation table."""
    # The indexes have to go before the columns they point at.
    for table, index_name in (
        ('irrigation_events', 'ix_irrigation_events_zone'),
        ('advisory_alerts', 'ix_advisory_alerts_zone'),
        ('advisory_alerts', 'ix_advisory_alerts_urgency'),
    ):
        if table in _existing_tables() and index_name in _existing_indexes(table):
            op.drop_index(index_name, table_name=table)

    for table, columns in (
        ('telemetry_readings', ['moisture_15cm_pct', 'moisture_30cm_pct', 'moisture_60cm_pct',
                                'soil_ec_us_cm', 'deep_percolation']),
        ('sensors', ['valve_id', 'moisture_15cm_pct', 'moisture_30cm_pct', 'moisture_60cm_pct',
                     'soil_ec_us_cm', 'deep_percolation']),
        ('advisory_alerts', ['zone', 'urgency']),
        ('irrigation_events', ['zone']),
        ('fields', ['language', 'quiet_hours_start', 'quiet_hours_end', 'digest_enabled',
                    'digest_day', 'digest_hour', 'escalation_phone']),
    ):
        if table not in _existing_tables():
            continue
        existing = _existing_columns(table)
        droppable = [name for name in columns if name in existing]
        if not droppable:
            continue
        with op.batch_alter_table(table, schema=None) as batch_op:
            for name in droppable:
                batch_op.drop_column(name)

    if 'actuation_events' in _existing_tables():
        op.drop_table('actuation_events')
