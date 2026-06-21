create extension if not exists timescaledb;

create table if not exists events (
  id uuid not null,
  stream_id text not null,
  entity_id text,
  ts timestamptz not null,
  event_type text not null,
  properties jsonb not null default '{}'::jsonb,
  inserted_at timestamptz not null default now(),
  primary key (id, ts)
);

select create_hypertable('events', 'ts', if_not_exists => true);

create index if not exists events_stream_ts_idx on events (stream_id, ts desc);
create index if not exists events_type_ts_idx on events (event_type, ts desc);
create index if not exists events_entity_ts_idx on events (entity_id, ts desc);
create index if not exists events_properties_gin_idx on events using gin (properties);
