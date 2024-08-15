-- Get all chunks of a hypertable
SELECT chunk_name, range_start, range_end, is_compressed FROM timescaledb_information.chunks WHERE hypertable_name = 'measurements';


-- Query for compressing all the chunks of a hypertable in a specific range
DO $BODY$
DECLARE chunk regclass;
DECLARE range_start_var text;
BEGIN
  FOR chunk, range_start_var IN SELECT format('%I.%I', chunk_schema, chunk_name)::regclass, range_start::text
  FROM timescaledb_information.chunks
  WHERE range_start > '2023-06-30' and range_start < '2024-01-10'
    and hypertable_name = 'measurements'
      order by range_start
  LOOP
    RAISE NOTICE 'Compress %', chunk::text;
    PERFORM compress_chunk(chunk::text::regclass);
    RAISE NOTICE 'with range %', range_start_var::text;
    COMMIT;
  END LOOP;
END
$BODY$;

-- Executing lttb
EXPLAIN ANALYZE
SELECT *
FROM unnest((
            SELECT lttb(ts, value, 2000)
             FROM measurements
             Where ts BETWEEN '2023-06-30 00:00:00+00:00'::TIMESTAMPTZ AND '2024-01-01 00:00:00+00:00'::TIMESTAMPTZ AND signal_id = 144986
             )
     );
