
### Polygon Token Transfer Indexer

A Rust-based Polygon blockchain indexer that monitors ERC20 token transfers (specifically POL token), tracks net flows for Binance addresses, and provides real-time cumulative statistics.

# Table of Contents

-   [Overview](#overview)
-   [Features](#features)
-   [Architecture](#architecture)
    - [Workflow](#workflow)
-   [Setup](#setup)
-   [Database Schema](#database-schema)
-   [Running the Project](#running-the-project)
-   [Modules & Code Explanation](#modules--code-explanation)
-   [Error & Exception Handling](#error--exception-handling)
-   [Limitations & Future Improvements](#limitations--future-improvements)



### Overview

This project continuously monitors the Polygon blockchain for ERC20 token transfers, focusing on a specific token (POL) and specific addresses (Binance). It stores raw transfer events in a SQLite database and calculates net inflows and outflows for tracking purposes.


### Features

1. Real-time monitoring of ERC20 transfers on Polygon
2. Tracks cumulative inflow, outflow, and net flow for Binance addresses
3. Stores raw transaction data in SQLite database
4. Robust retry logic for failed RPC requests
5. Command-line interface (CLI) for starting the indexer and querying net flows


### Architecture

    +--------------------+
    |  Polygon Blockchain|
    +--------------------+
              |
              v
      +------------------+
      |   RPC Provider   |
      +------------------+
              |
              v
    +---------------------------+
    |  Rust Indexer (CLI)       |
    |---------------------------|
    | - Fetch blocks            |
    | - Decode Transfer events  |
    | - Filter Binance addresses|
    | - Update net flows        |
    +---------------------------+
              |
              v
      +------------------+
      |  SQLite Database |
      +------------------+



## Workflow

- Fetch latest block number from Polygon RPC
- For each block, fetch ERC20 Transfer events for POL token
- Decode events, extract from, to, value
- Insert raw transfer into database
- Check if either from or to is a Binance address
- Update cumulative in/out and net flow in net_flows table



### Setup

**Requirements**

    Rust >= 1.80
    Cargo
    SQLite
    Internet access for Polygon RPC

**Install Dependencies**

    cargo build --release

**Environment Variables**

Create a .env file:

    DB_PATH=indexer.db
    POLYGON_RPC=https://polygon-rpc.com/


### Database Schema

**raw_transfers**

    | Column       | Type    | Description                         |
    | ------------ | ------- | ----------------------------------- |
    | tx_hash      | TEXT    | Transaction hash                    |
    | block_number | INTEGER | Block number                        |
    | from_address | TEXT    | Sender address                      |
    | to_address   | TEXT    | Receiver address                    |
    | value        | TEXT    | Transfer value (as string for U256) |
    | timestamp    | INTEGER | Block timestamp                     |



**net_flows**

    | Column         | Type    | Description                   |
    | -------------- | ------- | ----------------------------- |
    | id             | INTEGER | Primary key (always 1)        |
    | cumulative_in  | TEXT    | Sum of incoming transfers     |
    | cumulative_out | TEXT    | Sum of outgoing transfers     |
    | net_flow       | TEXT    | Difference between in and out |
    | last_updated   | INTEGER | Timestamp of last update      |



### Running the Project

**Start Indexer**

    cargo run -- start

- Fetches blocks continuously
- Decodes **ERC20** Transfer events
- Updates raw_transfers and net_flows


**Query Net Flows**

    cargo run -- query


**Output:**

    Cumulative In: 1000000000
    Cumulative Out: 500000000
    Net Flow: 500000000
    Last Updated Timestamp: 1758869965


### Modules & Code Explanation

1. **main.rs**

- CLI implemented using structopt (start / query)
- Initializes DB using **db::init_db**
- Fetches Polygon blocks using **ethers::Provider**
- Monitors POL token transfer events
- Filters Binance addresses and updates net flows

2. **db.rs**

- Handles SQLite operations
- **init_db:** reads schema.sql and creates tables
- **insert_raw_transfer:** stores raw transfer events
- **read_net_flows:** reads current net flow stats
- **update_net_flows_on_transfer**: updates cumulative in/out/net flows

3. **get_logs_with_retry**

- Implements retry logic for RPC failures
- Skips blocks after max retry attempts
- Handles transient network issues


### Error & Exception Handling

- Database errors are propagated via **anyhow::Result**
- RPC fetch failures retry 5 times with 5s delay
- U256 parsing errors handled safely (can be modified to throw explicit error)
- Invalid logs are skipped and logged



### Limitations & Future Improvements

- Currently monitors only one token (POL)
- Binance addresses are hardcoded; could be dynamic
- CLI-only interface; no web dashboard yet
- Parallel block fetching can improve performance
- Improved logging using tracing crate for better observability
