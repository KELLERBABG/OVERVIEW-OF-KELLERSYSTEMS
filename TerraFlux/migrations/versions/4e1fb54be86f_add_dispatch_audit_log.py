"""add dispatch audit log

Revision ID: 4e1fb54be86f
Revises: b1cd8787e1d0
Create Date: 2026-11-10 21:51:21.216268+00:00

"""
from typing import Sequence, Union

from alembic import op
import sqlalchemy as sa
import sqlmodel



revision: str = '4e1fb54be86f'
down_revision: Union[str, None] = 'b1cd8787e1d0'
branch_labels: Union[str, Sequence[str], None] = None
depends_on: Union[str, Sequence[str], None] = None


def upgrade() -> None:
    """Create the dispatch audit trail table (idempotent)."""
    inspector = sa.inspect(op.get_bind())
    if 'dispatch_logs' in inspector.get_table_names():
        return
    op.create_table('dispatch_logs',
    sa.Column('id', sa.Integer(), nullable=False),
    sa.Column('field_id', sa.Integer(), nullable=True),
    sa.Column('alert_id', sa.Integer(), nullable=True),
    sa.Column('channel', sqlmodel.sql.sqltypes.AutoString(), nullable=False),
    sa.Column('target', sqlmodel.sql.sqltypes.AutoString(), nullable=False),
    sa.Column('status', sqlmodel.sql.sqltypes.AutoString(), nullable=False),
    sa.Column('detail', sqlmodel.sql.sqltypes.AutoString(), nullable=True),
    sa.Column('attempts', sa.Integer(), nullable=False),
    sa.Column('created_at', sa.DateTime(), nullable=False),
    sa.ForeignKeyConstraint(['alert_id'], ['advisory_alerts.id'], ),
    sa.ForeignKeyConstraint(['field_id'], ['fields.id'], ),
    sa.PrimaryKeyConstraint('id')
    )
    with op.batch_alter_table('dispatch_logs', schema=None) as batch_op:
        batch_op.create_index(batch_op.f('ix_dispatch_logs_alert_id'), ['alert_id'], unique=False)
        batch_op.create_index(batch_op.f('ix_dispatch_logs_channel'), ['channel'], unique=False)
        batch_op.create_index(batch_op.f('ix_dispatch_logs_created_at'), ['created_at'], unique=False)
        batch_op.create_index(batch_op.f('ix_dispatch_logs_field_id'), ['field_id'], unique=False)
        batch_op.create_index(batch_op.f('ix_dispatch_logs_status'), ['status'], unique=False)

    # ### end Alembic commands ###


def downgrade() -> None:
    inspector = sa.inspect(op.get_bind())
    if 'dispatch_logs' not in inspector.get_table_names():
        return
    with op.batch_alter_table('dispatch_logs', schema=None) as batch_op:
        batch_op.drop_index(batch_op.f('ix_dispatch_logs_status'))
        batch_op.drop_index(batch_op.f('ix_dispatch_logs_field_id'))
        batch_op.drop_index(batch_op.f('ix_dispatch_logs_created_at'))
        batch_op.drop_index(batch_op.f('ix_dispatch_logs_channel'))
        batch_op.drop_index(batch_op.f('ix_dispatch_logs_alert_id'))

    op.drop_table('dispatch_logs')
    # ### end Alembic commands ###
