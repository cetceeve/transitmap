import functions_framework
from zipfile import ZipFile
from urllib.request import urlopen, Request
import numpy as np
import pandas as pd
from io import BytesIO
from google.cloud import storage
from datetime import datetime
import os


@functions_framework.cloud_event
def aggregate_gtfs_static_sweden(cloud_event):
    TRAFIKLAB_GTFS_STATIC_KEY = os.environ.get(
        "TRAFIKLAB_GTFS_STATIC_KEY", "Specified environment variable is not set."
    )
    print(TRAFIKLAB_GTFS_STATIC_KEY)
    print("downloading gtfs static sweden...")
    req = Request(
        f"https://opendata.samtrafiken.se/gtfs-sweden/sweden.zip?key=35005202da12483f992411c4a694d86a"
    )
    req.add_header("accept", "application/octet-stream")
    req.add_header("accept-encoding", "gzip")
    resp = urlopen(req)
    myzip = ZipFile(BytesIO(resp.read()))
    print("downloaded raw data")

    # For local development
    # myzip = ZipFile("./Archive.zip")

    with myzip.open("trips.txt") as f:
        trips_df = pd.read_csv(
            f,
            dtype={
                "route_id": str,
                "service_id": np.int64,
                "trip_id": np.int64,
                "direction_id": np.int64,
                "shape_id": np.int64,
                "trip_headsign": str,
            },
        )
        trips_df = trips_df[["route_id", "trip_id", "shape_id", "direction_id"]]

    with myzip.open("routes.txt") as f:
        routes_df = pd.read_csv(
            f,
            dtype={
                "route_id": str,
                "agency_id": np.int64,
                "route_type": np.int64,
                "route_short_name": str,
                "route_long_name": str,
                "route_desc": str,
            },
        )
        routes_df = routes_df[
            [
                "route_id",
                "agency_id",
                "route_type",
                "route_short_name",
                "route_long_name",
            ]
        ]

    with myzip.open("agency.txt") as f:
        agency_df = pd.read_csv(
            f,
            dtype={
                "agency_id": np.int64,
                "agency_name": str,
                "agency_url": str,
                "agency_timezone": str,
                "agency_lang": str,
                "agency_fare_url": str,
            },
        )
        agency_df = agency_df[["agency_id", "agency_name"]]

    with myzip.open("stop_times.txt") as f:
        stop_times_df = pd.read_csv(
            f,
            dtype={
                "trip_id": np.int64,
                "stop_id": np.int64,
                "stop_sequence": np.int64,
                "arrival_time": str,
                "departure_time": str,
                "stop_headsign": str,
                "shape_dist_traveled": np.float64,
            },
        )
        stop_times_df.rename(columns={"stop_headsign": "trip_headsign"}, inplace=True)

    with myzip.open("stops.txt") as f:
        stops_df = pd.read_csv(
            f,
            dtype={
                "stop_id": np.int64,
                "stop_name": str,
                "stop_lat": np.float64,
                "stop_lon": np.float64,
            },
        )
        stops_df = stops_df[["stop_id", "stop_name", "stop_lat", "stop_lon"]]

    # create df with stop sequence for each route
    # we don't merge directly on trips because trips could skip stops
    # using this all routes will have the same stops
    # when we merge the departure and arrival times on trop_id some stops will have NaN
    # stop_seq_df = trips_df \
    #     .merge(stop_times_df, on="trip_id") \
    #     .drop_duplicates(subset=["route_id", "direction_id", "stop_sequence"]) \
    #     .merge(stops_df, on="stop_id")
    # stop_seq_df = stop_seq_df[["route_id", "direction_id", "trip_headsign", "stop_id", "stop_name", "stop_lat", "stop_lon", "stop_sequence", "shape_dist_traveled"]]

    aggregated_df = (
        trips_df.merge(routes_df, on="route_id", how="left")
        .merge(agency_df, on="agency_id", how="left")
        .merge(stop_times_df, on="trip_id", how="left")
        .merge(stops_df, on="stop_id", how="left")
    )
    # .merge(stop_seq_df, on=["route_id", "direction_id"], how="left") \

    # Sort the sequence ids
    aggregated_df.sort_values(["trip_id", "stop_sequence"], inplace=True)

    # print("trips: ", trips_df.shape)
    # print("aggregated: ", aggregated_df.shape)
    # print(f"With Arrival Time: {aggregated_df[['arrival_time']].count().iloc[0]}/{aggregated_df.shape[0]}")

    # function to represent list of values
    def serialize_series(series):
        return "|".join(str(x) for x in list(series))

    # Store all stops as lists in the csv to reduce file size and easy merging
    final_df = aggregated_df.groupby(by=["trip_id"], as_index=False).agg(
        {
            "shape_id": "first",
            "route_id": "first",
            "direction_id": "first",
            "route_type": "first",
            "route_short_name": "first",
            "route_long_name": "first",
            "trip_headsign": "first",
            "agency_id": "first",
            "agency_name": "first",
            "stop_id": serialize_series,
            "stop_name": serialize_series,
            "stop_lat": serialize_series,
            "stop_lon": serialize_series,
            "stop_sequence": serialize_series,
            "arrival_time": serialize_series,
            "departure_time": serialize_series,
            "shape_dist_traveled": serialize_series,
        }
    )

    print("Final: ", final_df.shape)
    # print(final_df)
    # print(final_df.columns)

    # Compress output to safe over 90% storage space
    # time or writing: 860MB uncompressed, 57MB compressed
    contents = BytesIO()
    final_df.to_csv(contents, index=False, encoding="utf-8", compression="gzip")
    # with open("test.csv.gz", mode="wb") as f:
    #     f.write(contents.getbuffer())

    storage_client = storage.Client()
    bucket = storage_client.bucket("gtfs_static")
    blob = bucket.blob("sweden_aggregated_metadata.csv.gz")
    blob.upload_from_file(contents, rewind=True)

    print("======================================")
    print("refreshed static GTFS Sweden aggregate")
    print("======================================")
