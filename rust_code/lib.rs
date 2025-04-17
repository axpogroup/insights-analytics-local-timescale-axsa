use pgrx::prelude::*;
use std::collections::BTreeSet;

pgrx::pg_module_magic!();

#[derive(Debug, Clone)]
struct Signal {
    ts: Vec<f64>,
    values: Vec<f64>,
}

fn lttb(signal: &Signal, threshold: usize) -> Signal {
    let data_length = signal.ts.len();
    if threshold >= data_length || threshold == 0 {
        return signal.clone();
    }

    let mut downsampled_ts = Vec::new();
    let mut downsampled_values = Vec::new();
    let every = (data_length - 2) as f64 / (threshold - 2) as f64;
    let mut a = 0;

    downsampled_ts.push(signal.ts[a]);
    downsampled_values.push(signal.values[a]);

    for i in 0..(threshold - 2) {
        let avg_range_start = ((i + 1) as f64 * every + 1.0).floor() as usize;
        let avg_range_end = (((i + 2) as f64 * every + 1.0).floor() as usize).min(data_length);
        if avg_range_start >= avg_range_end {
            continue;
        }

        let avg_time = signal.ts[avg_range_start..avg_range_end].iter().copied().sum::<f64>() / (avg_range_end - avg_range_start) as f64;
        let avg_signal = signal.values[avg_range_start..avg_range_end].iter().copied().sum::<f64>() / (avg_range_end - avg_range_start) as f64;

        let range_offs = (i as f64 * every + 1.0).floor() as usize;
        let range_to = (((i + 1) as f64 * every + 1.0).floor() as usize).min(data_length);

        let mut max_area = -1.0;
        let mut next_a = a;

        for j in range_offs..range_to {
            if j >= signal.ts.len() || j >= signal.values.len() {
                continue;
            }
            let area = ((signal.ts[a] - avg_time) * (signal.values[j] - signal.values[a])
                - (signal.ts[a] - signal.ts[j]) * (avg_signal - signal.values[a]))
                .abs() * 0.5;
            if area > max_area {
                max_area = area;
                next_a = j;
            }
        }

        downsampled_ts.push(signal.ts[next_a]);
        downsampled_values.push(signal.values[next_a]);
        a = next_a;
    }

    downsampled_ts.push(signal.ts[data_length - 1]);
    downsampled_values.push(signal.values[data_length - 1]);

    Signal {
        ts: downsampled_ts,
        values: downsampled_values,
    }
}

fn find_last_and_sum(signals: &[Signal], points: &Signal) -> Signal {
    let mut summed_values = vec![0.0; points.ts.len()];

    for signal in signals {
        if signal.ts.is_empty() || signal.ts.len() != signal.values.len() {
            pgrx::warning!("Skipping invalid signal with ts.len={} and values.len={}", signal.ts.len(), signal.values.len());
            continue;
        }

        for (i, &t) in points.ts.iter().enumerate() {
            let result = signal.ts.binary_search_by(|x| x.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Less));
            let value = match result {
                Ok(pos) => {
                    signal.values.get(pos).copied().unwrap_or(0.0)
                },
                Err(pos) => {
                    if pos == 0 {
                        0.0
                    } else {
                        signal.values.get(pos - 1).copied().unwrap_or(0.0)
                    }
                }
            };
            summed_values[i] += value;
        }
    }

    Signal {
        ts: points.ts.clone(),
        values: summed_values,
    }
}

#[pg_extern]
fn process_multiple_signals<'a>(
    ts_list: Array<'a, Array<'a, f64>>,
    values_list: Array<'a, Array<'a, f64>>,
    threshold: i32,
) -> TableIterator<'a, (name!(timestamp, f64), name!(value, f64))> {
    let threshold = if threshold <= 0 { 2 } else { threshold as usize };

    let signals: Vec<Signal> = ts_list
        .iter()
        .zip(values_list.iter())
        .filter_map(|(ts_opt, val_opt)| {
            let ts_array = ts_opt?;
            let val_array = val_opt?;

            let ts: Vec<f64> = ts_array.iter().filter_map(|x| x).collect();
            let values: Vec<f64> = val_array.iter().filter_map(|x| x).collect();

            if ts.len() != values.len() || ts.is_empty() {
                pgrx::warning!("Invalid input: ts.len = {}, values.len = {}", ts.len(), values.len());
                return None;
            }

            Some(Signal { ts, values })
        })
        .collect();

    if signals.is_empty() {
        return TableIterator::new(vec![].into_iter());
    }

    let resampled_signals: Vec<Signal> = signals.iter().map(|s| lttb(s, threshold)).collect();

    let mut ts_set = BTreeSet::new();
    let scale = 1_000_000.0;
    for sig in &resampled_signals {
        for &t in &sig.ts {
            if t.is_finite() {
                ts_set.insert((t * scale).round() as i64);
            }
        }
    }

    let all_ts: Vec<f64> = ts_set.into_iter().map(|t| t as f64 / scale).collect();
    let all_points = Signal {
        ts: all_ts.clone(),
        values: vec![0.0; all_ts.len()],
    };

    let result_signal = find_last_and_sum(&signals, &all_points);
    let results: Vec<(f64, f64)> = result_signal.ts.into_iter()
        .zip(result_signal.values)
        .filter(|(t, v)| t.is_finite() && v.is_finite())
        .collect();

    TableIterator::new(results.into_iter())
}









