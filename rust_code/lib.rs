use pgrx::prelude::*;
use std::collections::HashSet;

pgrx::pg_module_magic!();

#[derive(Clone, Debug)]
struct Signal {
    ts: Vec<f64>,
    values: Vec<f64>,
}

// LTTB downsampling function
fn lttb(signal: &Signal, threshold: usize) -> Signal {
    let data_length = signal.ts.len();
    if threshold >= data_length || threshold == 0 {
        return signal.clone(); // No need to downsample
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

        let avg_time = signal.ts[avg_range_start..avg_range_end].iter().copied().sum::<f64>()
            / (avg_range_end - avg_range_start) as f64;
        let avg_signal = signal.values[avg_range_start..avg_range_end]
            .iter()
            .copied()
            .sum::<f64>()
            / (avg_range_end - avg_range_start) as f64;

        let range_offs = (i as f64 * every + 1.0).floor() as usize;
        let range_to = (((i + 1) as f64 * every + 1.0).floor() as usize).min(data_length);

        let mut max_area = -1.0;
        let mut next_a = 0;

        for j in range_offs..range_to {
            let area = ((signal.ts[a] - avg_time) * (signal.values[j] - signal.values[a])
                - (signal.ts[a] - signal.ts[j]) * (avg_signal - signal.values[a]))
                .abs()
                * 0.5;
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

#[pg_extern]
fn find_last_and_sum(signal: Signal, points: Signal) -> Signal {
    let mut summed_values = Vec::new();

    for (i, &timestamp) in points.ts.iter().enumerate() {
        let mut index = signal
            .ts
            .binary_search_by(|&probe| probe.partial_cmp(&timestamp).unwrap())
            .unwrap_or_else(|x| x.saturating_sub(1));

        if index >= signal.ts.len() {
            index = signal.ts.len() - 1;
        }

        let total_value = signal.values[index] + points.values[i];
        summed_values.push(total_value);
    }

    Signal {
        ts: points.ts.clone(),
        values: summed_values,
    }
}

// Process two signals and return a merged downsampled result
#[pg_extern]
fn process_two_signals(signal_1: Signal, signal_2: Signal, threshold: i32) -> Signal {
    let resampled_signal_1 = lttb(&signal_1, threshold as usize);
    let resampled_signal_2 = lttb(&signal_2, threshold as usize);

    let summed_signal_1 = find_last_and_sum(signal_2.clone(), resampled_signal_1.clone());
    let summed_signal_2 = find_last_and_sum(signal_1.clone(), resampled_signal_2.clone());

    let scale_factor = 1_000_000;
    let mut unique_ts: HashSet<i64> = HashSet::new();
    for &ts in &summed_signal_1.ts {
        unique_ts.insert((ts * scale_factor as f64).round() as i64);
    }
    for &ts in &summed_signal_2.ts {
        unique_ts.insert((ts * scale_factor as f64).round() as i64);
    }

    let mut merged_ts: Vec<f64> = unique_ts
        .into_iter()
        .map(|ts| ts as f64 / scale_factor as f64)
        .collect();
    merged_ts.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut merged_values = Vec::new();
    for &timestamp in &merged_ts {
        let value_1 = summed_signal_1
            .ts
            .iter()
            .position(|&t| (t * scale_factor as f64).round() as i64 == (timestamp * scale_factor as f64).round() as i64)
            .map(|i| summed_signal_1.values[i]);

        let value_2 = summed_signal_2
            .ts
            .iter()
            .position(|&t| (t * scale_factor as f64).round() as i64 == (timestamp * scale_factor as f64).round() as i64)
            .map(|i| summed_signal_2.values[i]);

        let combined_value = match (value_1, value_2) {
            (Some(v1), Some(_v2)) => v1,
            (Some(v1), None) => v1,
            (None, Some(v2)) => v2,
            (None, None) => 0.0,
        };

        merged_values.push(combined_value);
    }

    Signal {
        ts: merged_ts,
        values: merged_values,
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use super::*;

    #[pg_test]
    fn test_process_two_signals() {
        let signal_1 = Signal {
            ts: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            values: vec![0.0, 1.0, 0.0, -1.0, 0.0],
        };
        let signal_2 = Signal {
            ts: vec![0.0, 1.5, 3.0, 4.5],
            values: vec![1.0, 0.5, -0.5, -1.0],
        };

        let result = process_two_signals(signal_1, signal_2, 3);

        assert_eq!(result.ts.len(), 6);
    }
}

/// Required for `cargo pgrx test`
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
