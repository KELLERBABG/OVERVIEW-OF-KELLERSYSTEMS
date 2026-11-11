"""add irrigation zone to sensors

Revision ID: b1cd8787e1d0
Revises: 46c73bee87d9
Create Date: 2026-11-10 21:48:01.057666+00:00

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa
import sqlmodel



revision: str = 'b1cd8787e1d0'
down_revision: Union[str, None] = '46c73bee87d9'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    """Add the optional irrigation zone to the sensors table.

    The operation is guarded so it stays idempotent for deployments that already
    applied the legacy in-app column backfill (src/api/database.py).
    """
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = {col['name'] for col in inspector.get_columns('sensors')}
    if 'zone' in columns:
        return
    with op.batch_alter_table('sensors', schema=None) as batch_op:
        batch_op.add_column(sa.Column('zone', sqlmodel.sql.sqltypes.AutoString(), nullable=True))
        batch_op.create_index(batch_op.f('ix_sensors_zone'), ['zone'], unique=False)


def downgrade() -> None:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = {col['name'] for col in inspector.get_columns('sensors')}
    if 'zone' not in columns:
        return
    with op.batch_alter_table('sensors', schema=None) as batch_op:
        batch_op.drop_index(batch_op.f('ix_sensors_zone'))
        batch_op.drop_column('zone')
