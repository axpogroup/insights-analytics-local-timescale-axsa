# Project Overview

## Scalable and Flexible Downsampling of Time-series Data
A time-series database system (TSDBS) is designed to handle huge amounts of data that are time-stamped with time points. 
Such data must be tracked, stored, downsampled, and aggregated over time so that the data can be analyzed efficiently. 
A TSDBS leverages the specific characteristics of time series data to store and process data more efficiently than 
general-purpose database systems.
Timescale https://github.com/timescale/timescaledb is an open-source database system designed to make database 
technology scalable for time-series data and is implemented as a PostgreSQL extension. One component of Timescale is 
the Largest Triangle Three Buckets (LTTB) method 
https://github.com/timescale/timescaledb-toolkit/blob/main/extension/src/lttb.rs, a downsampling algorithm that tries to
retain visual similarity bet- ween the downsampled data and the original dataset. The idea of LLTB is to select data
points that form the largest triangular area with a previously selected data point and the average value 
of the next bin. Although LTTB is effective, there are limitations that shall be investigated and improved in
this Master project.

### Task 1: 
Scalable LLTB LTTB becomes inefficient when applied to high-frequency data over extended periods.
The goal of this task is to enhance the performance of LTTB under these conditions.
This paper https://arxiv.org/abs/2305.00332 proposes to use a preselection of points for this.

### Task 2: 
Aggregated LLTB In its current form, LTTB allows visualization of individual timeseries signals separately.
However, in applications such as monitoring water inflow to a hydro dam, where multiple inflows exist, 
it would be advantageous to aggregate and visualize these signals together. An extension that allows operations like 
lttb(sum(sig1, sig2, sig3)) or more generally lttb(func(x1, x2, x3,...)), where func could be any aggregation function
like sum, minimum, maximum, or average, shall be implemented.


### Optional: Task 3: 
Multi-Dimensional LLTB Presently, LTTB is limited to processing a single signal at a time (e.g., lttb(sig1)). 
The goal is expand LTTB to handle multiple signals simultaneously (e.g., lttb(sig1, sig2, ...)), 
enabling more advanced and informative visualizations.


# Local timescale db
For testing porposes we have a docker-compose file that will start a timescale db instance.

## Setup DB 

```bash
docker docker compose up --build 
```

## Uploading data

You can upload data by just executing the data_insert.py file. To run the python file, we recommend first creating and activating a venv
```
python3 -m venv venv
source venv/bin/activate
```

and then installing the requirements with 
```
pip install -r requirements.txt
```

Before you can upload the data (executing the data_insert.py file) you need to create a folder called data and put the 
csv files in there.

After uploading the data you need to compress the chunks
first in order to get good performance. See helpful_queries.sql for more information.

Have a look at the documentation of the [timescaledb](https://docs.timescale.com/timescaledb/latest/overview) for more information.


## Data to start with 

We have currently two signals in the csv. One frequent signal and one infrequent signal. Currently, executing lttb on the 
high frequent data is not efficient and takes a lot of time. 

- 144986 --> frequent signal
- 11883 --> infrequent signal$

# Postgres & Rust

Check out here: https://kaiwern.com/posts/2022/07/20/writing-postgresql-extension-in-rust-with-pgx/

We build a postgres extension called my_extension. The function can be defined in lib.rs file. 
Everytime you change there something you need to build the docker image new. So you need to make sure that your rust 
code is correct and compiles.


## Build the docker image

If you change something in the rust code you need to build the docker image new.
```bash
 docker build --no-cache -t test_rust:tag1 . 
```





