-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

CREATE EXTENSION timescaledb_toolkit;

ALTER EXTENSION timescaledb_toolkit UPDATE;

-- Create the measurement table
CREATE TABLE measurements
(
    Ts               TIMESTAMPTZ      NOT NULL,
    signal_id         INTEGER          NOT NULL,
    Value DOUBLE PRECISION
);

-- Convert the table to a hypertable
SELECT create_hypertable(
    'measurements',
    'ts',
    chunk_time_interval => INTERVAL '1 day'
);

-- Define unique index
CREATE UNIQUE INDEX idx_sensorid_time
    ON measurements(signal_id, ts);

-- Define compression policy
ALTER TABLE measurements
    SET (
        timescaledb.compress,
        timescaledb.compress_orderby = 'ts DESC',
        timescaledb.compress_segmentby = 'signal_id'
);

-- Define chunk time interval
SELECT set_chunk_time_interval('measurements', INTERVAL '24 hours');

