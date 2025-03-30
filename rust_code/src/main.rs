use polars::prelude::*;
use ndarray::Array1;
use plotters::prelude::*;
use plotters::style::{BLACK, BLUE, CYAN, GREEN, MAGENTA, RED};
use std::time::Instant;
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;


// LTTB downsampling
fn lttb(dataframe: &DataFrame, threshold: usize) -> DataFrame {
    let data_length = dataframe.height();
    if threshold >= data_length || threshold == 0 {
        return dataframe.clone();
    }

    let ts: Vec<f64> = dataframe.column("ts").unwrap().f64().unwrap().into_iter().flatten().collect();
    let values: Vec<f64> = dataframe.column("values").unwrap().f64().unwrap().into_iter().flatten().collect();

    let data: Vec<(f64, f64)> = ts.into_iter().zip(values.into_iter()).collect();
    let every = (data_length - 2) as f64 / (threshold - 2) as f64;
    let mut max_area_point = vec![data[0]];
    let mut a = 0;
    let mut next_a = 0;

    for i in 0..(threshold - 2) {
        let avg_range_start = ((i as f64 + 1.0) * every).floor() as usize + 1;
        let avg_range_end = ((i as f64 + 2.0) * every).floor() as usize + 1;
        let avg_range_end = avg_range_end.min(data_length);

        let avg_time: f64 = data[avg_range_start..avg_range_end].iter().map(|d| d.0).sum::<f64>() / (avg_range_end - avg_range_start) as f64;
        let avg_signal: f64 = data[avg_range_start..avg_range_end].iter().map(|d| d.1).sum::<f64>() / (avg_range_end - avg_range_start) as f64;

        let range_offs = ((i as f64) * every).floor() as usize + 1;
        let range_to = ((i as f64 + 1.0) * every).floor() as usize + 1;

        let mut max_area = -1.0;
        for j in range_offs..range_to {
            let area = ((data[a].0 - avg_time) * (data[j].1 - data[a].1) - (data[a].0 - data[j].0) * (avg_signal - data[a].1)).abs() * 0.5;
            if area > max_area {
                max_area = area;
                next_a = j;
            }
        }

        max_area_point.push(data[next_a]);
        a = next_a;
    }

    max_area_point.push(data[data_length - 1]);

    DataFrame::new(vec![
        Series::new("ts", max_area_point.iter().map(|(t, _)| *t).collect::<Vec<_>>()),
        Series::new("values", max_area_point.iter().map(|(_, v)| *v).collect::<Vec<_>>()),
    ]).unwrap()
}

fn find_last_and_sum(signals: &[DataFrame], points: &DataFrame) -> DataFrame {
    let start_time = Instant::now();

    let resampled_timestamps: Vec<f64> = points.column("ts").unwrap().f64().unwrap().into_iter().flatten().collect();
    let mut summed_values = vec![0.0; resampled_timestamps.len()];

    for signal in signals {
        let ts_values: Vec<f64> = signal.column("ts").unwrap().f64().unwrap().into_iter().flatten().collect();
        let signal_values: Vec<f64> = signal.column("values").unwrap().f64().unwrap().into_iter().flatten().collect();

        for (i, &t) in resampled_timestamps.iter().enumerate() {
            let result = ts_values.binary_search_by(|x| x.partial_cmp(&t).unwrap());

            let value = match result {
                Ok(pos) => signal_values[pos],
                Err(0) => 0.0, // cannot find the last timestamp，use 0
                Err(pos) => signal_values[pos - 1],
            };

            summed_values[i] += value;
        }
    }
    let elapsed_time = start_time.elapsed();
    println!("Summation time for current operation: {:.4?} seconds", elapsed_time);

    DataFrame::new(vec![
        Series::new("ts", resampled_timestamps),
        Series::new("summed_value_combined", summed_values),
    ]).unwrap()
}

fn process_multiple_signals(signals: &[DataFrame], threshold: usize) -> DataFrame {
    let start_time = Instant::now(); // record the start time

    let resampled_signals: Vec<DataFrame> = signals.iter().map(|s| lttb(s, threshold)).collect();

    let mut all_timestamps: Vec<f64> = resampled_signals.iter()
        .flat_map(|df| df.column("ts").unwrap().f64().unwrap().into_iter().flatten())
        .collect();
    
    all_timestamps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    all_timestamps.dedup();

    let all_timestamps_df = DataFrame::new(vec![
        Series::new("ts", all_timestamps.clone()),
        Series::new("values", vec![0.0; all_timestamps.len()]),
    ]).unwrap();

    let merged_df = find_last_and_sum(signals, &all_timestamps_df);

    let elapsed_time = start_time.elapsed(); // calculate running time
    println!("Total processing time: {:.4?} seconds", elapsed_time);

    merged_df
}

// Plot
fn plot_multiple_signals(signals: &[DataFrame], threshold: usize) {
    let merged_df = process_multiple_signals(signals, threshold);
    let ts_summed: Vec<f64> = merged_df.column("ts").unwrap().f64().unwrap().into_iter().flatten().collect();
    let summed_values: Vec<f64> = merged_df.column("summed_value_combined").unwrap().f64().unwrap().into_iter().flatten().collect();

    let root = BitMapBackend::new("plot.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Signals and Summed Values", ("sans-serif", 20))
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0f64..10f64, -5f64..5f64)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    let colors = [RED, BLUE, GREEN, MAGENTA, CYAN];

    for (i, signal) in signals.iter().enumerate() {
        let ts: Vec<f64> = signal.column("ts").unwrap().f64().unwrap().into_iter().flatten().collect();
        let values: Vec<f64> = signal.column("values").unwrap().f64().unwrap().into_iter().flatten().collect();
        chart.draw_series(LineSeries::new(ts.into_iter().zip(values.into_iter()), &colors[i]))
            .unwrap()
            .label(format!("Signal {}", i + 1));
    }

    let mut original_sum_map: BTreeMap<OrderedFloat<f64>, f64> = BTreeMap::new();
    for signal in signals {
        let ts: Vec<f64> = signal.column("ts").unwrap().f64().unwrap().into_iter().flatten().collect();
        let values: Vec<f64> = signal.column("values").unwrap().f64().unwrap().into_iter().flatten().collect();
        for (t, v) in ts.iter().zip(values.iter()) {
            *original_sum_map.entry(OrderedFloat(*t)).or_insert(0.0) += v;
        }
    }

    let (original_ts, original_values): (Vec<f64>, Vec<f64>) = original_sum_map
        .into_iter()
        .map(|(t, v)| (t.into_inner(), v))  
        .unzip();

    if !original_ts.is_empty() {
        chart.draw_series(LineSeries::new(
            original_ts.clone().into_iter().zip(original_values.into_iter()),
            ShapeStyle {
                color: BLACK.to_rgba(),
                filled: false,
                stroke_width: 2,
            },
        ))
        .unwrap()
        .label("Original_SUM");
    }

    chart.draw_series(LineSeries::new(
        ts_summed.clone().into_iter().zip(summed_values.into_iter()),
        ShapeStyle {
            color: RED.to_rgba(),
            filled: false,
            stroke_width: 2,
        },
    ))
    .unwrap()
    .label("Summed Combined Line");

    chart.configure_series_labels().border_style(&BLACK).draw().unwrap();

    root.present().unwrap();
    println!("Plot saved as 'plot.png'");
}


// main function
 
fn main() {
    let number = 2000000;
    let time: Vec<f64> = (0..number).map(|i| (i as f64) * (10.0 / number as f64)).collect();
    let time_arr = Array1::from(time);

    let signals = vec![
        time_arr.mapv(|t| t.sin()),
        time_arr.mapv(|t| t.cos()),
        time_arr.mapv(|t| (2.0 * t.sin())),
        time_arr.mapv(|t| (2.0 * t.cos())),
    ];

    let signal_dfs: Vec<DataFrame> = signals.into_iter()
        .map(|sig| DataFrame::new(vec![Series::new("ts", time_arr.to_vec()), Series::new("values", sig.to_vec())]).unwrap())
        .collect();

    plot_multiple_signals(&signal_dfs, 30);
}


/* 
fn main() {
    let number = 100_000; 
    let time: Vec<f64> = (0..number).map(|i| (i as f64) * (10.0 / number as f64)).collect();
    let time_arr = Array1::from(time);

    // define signals
    let base_signals = vec![
        |t: f64| t.sin(),
        |t: f64| t.cos(),
        |t: f64| (2.0 * t.sin()),
        |t: f64| (2.0 * t.cos()),
        |t: f64| (3.0 * t.sin()),
        |t: f64| (3.0 * t.cos()),
        |t: f64| (4.0 * t.sin()),
        |t: f64| (4.0 * t.cos()),
        |t: f64| (5.0 * t.sin()),
        |t: f64| (5.0 * t.cos()),
    ];

    let mut results = Vec::new();

    for signal_count in 2..=10 {
        let signals: Vec<Array1<f64>> = base_signals.iter().take(signal_count)
            .map(|f| time_arr.mapv(|t| f(t)))
            .collect();

        let signal_dfs: Vec<DataFrame> = signals.into_iter()
            .map(|sig| DataFrame::new(vec![Series::new("ts", time_arr.to_vec()), Series::new("values", sig.to_vec())]).unwrap())
            .collect();

        let start_time = Instant::now();
        process_multiple_signals(&signal_dfs, 30);
        let elapsed_time = start_time.elapsed().as_secs_f64();

        results.push((signal_count, elapsed_time));
    }
    println!("Signals | Processing Time (seconds)");
    println!("-------------------------------");
    for (num_signals, time) in &results {
        println!("{:<8} | {:.6}", num_signals, time);
    }

    let mut file = File::create("performance_results.csv").expect("Failed to create file");
    writeln!(file, "Signals,Processing Time (seconds)").unwrap();
    for (num_signals, time) in &results {
        writeln!(file, "{},{}", num_signals, time).unwrap();
    }

    println!("Results saved to 'performance_results.csv'");
}
*/
/* 
fn measure_runtime() {
    let data_sizes = vec![
        10000, 20000, 40000, 60000, 80000, 120000, 
        160000, 200000, 240000, 280000, 320000
    ];
    
    let mut results: Vec<(usize, f64)> = Vec::new();

    for &number in &data_sizes {
        let time: Vec<f64> = (0..number).map(|i| (i as f64) * (10.0 / number as f64)).collect();
        let time_arr = Array1::from(time);

        let signals = vec![
            time_arr.mapv(|t| t.sin()),
            time_arr.mapv(|t| t.cos()),
            time_arr.mapv(|t| (2.0 * t.sin())),
            time_arr.mapv(|t| (2.0 * t.cos())),
        ];

        let signal_dfs: Vec<DataFrame> = signals.into_iter()
            .map(|sig| DataFrame::new(vec![Series::new("ts", time_arr.to_vec()), Series::new("values", sig.to_vec())]).unwrap())
            .collect();

        let start_time = Instant::now();
        process_multiple_signals(&signal_dfs, 30);
        let elapsed_time = start_time.elapsed().as_secs_f64();

        results.push((number, elapsed_time));
    }

    plot_runtime_chart(&results);
}

fn plot_runtime_chart(results: &[(usize, f64)]) {
    let root = BitMapBackend::new("runtime_plot.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let max_x = results.iter().map(|(x, _)| *x).max().unwrap() as f64;
    let max_y = results.iter().map(|(_, y)| *y).fold(0.0, f64::max);

    let mut chart = ChartBuilder::on(&root)
        .caption("Processing Time vs. Data Size", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0f64..max_x, 0f64..max_y)
        .unwrap();

    chart.configure_mesh()
        .x_desc("Number of Data Points")
        .y_desc("Processing Time (s)")
        .draw().unwrap();

    chart.draw_series(LineSeries::new(
        results.iter().map(|(x, y)| (*x as f64, *y)),
        &BLUE,
    )).unwrap()
    .label("Processing Time")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart.configure_series_labels()
        .border_style(&BLACK)
        .draw()
        .unwrap();

    root.present().unwrap();
    println!("Runtime chart saved as 'runtime_plot.png'");
}

fn main() {
    measure_runtime();
}
*/
