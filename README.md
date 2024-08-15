# Local timescale db

## Setup DB 

```bash
docker compose -f docker-compose.yml up
```

## Uploading data

You can upload data by just executing the data_insert.py file. After uploading the data you need to compress the chunks
first in order to get good performance. 

Have a look at the documentation of the [timescaledb](https://docs.timescale.com/timescaledb/latest/overview) for more information.


## Data to start with 

We have currently two signals in the csv. On frequent signal and one infrequent signal.
- 144986 --> frequent signal
- 11883 --> infrequent signal
