import pandas as pd
import psycopg2
from psycopg2 import sql
from psycopg2.extras import execute_values

if __name__ == "__main__":
    # Database connection parameters
    DB_HOST = "localhost"
    DB_PORT = "5432"
    DB_NAME = "postgres"
    DB_USER = "postgres"
    DB_PASSWORD = "yourpassword"

    # CSV file path
    CSV_FILE_PATH = "data/raw_data.csv"

    # Read the CSV file into a pandas DataFrame
    df = pd.read_csv(CSV_FILE_PATH)
    df = df.dropna()
    print(df.shape)

    df_reformat = pd.DataFrame()
    df_reformat['ts'] = df['timestamp']
    df_reformat['signal_id'] = df['signal_id']
    df_reformat['value'] = df['value']

    # Establish a connection to the PostgreSQL database
    conn = psycopg2.connect(
        host=DB_HOST,
        port=DB_PORT,
        dbname=DB_NAME,
        user=DB_USER,
        password=DB_PASSWORD
    )

    # Create a cursor object
    cur = conn.cursor()

    # Insert data into the PostgreSQL table
    data_tuples = [tuple(x) for x in df_reformat.to_numpy()]

    # Define the insert query
    insert_query = sql.SQL("""
        INSERT INTO measurements (ts, signal_id, value)
        VALUES %s
    """)

    # Use execute_values to perform batch insert
    execute_values(cur, insert_query, data_tuples)

    # Commit the transaction
    conn.commit()

    # Close the cursor and connection
    cur.close()
    conn.close()

    print("Data inserted successfully!")
