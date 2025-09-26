mod db;
use db::{init_db, insert_raw_transfer, update_net_flows_on_transfer, read_net_flows};

use ethers::prelude::*;
use ethers::types::H160;
use structopt::StructOpt;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use ethers::contract::EthEvent;
use ethers::abi::RawLog;

#[derive(Debug, StructOpt)]
enum Command {
    Start,
    Query,
}

// ERC20 Transfer event
abigen!(
    PolToken,
    r#"[ event Transfer(address indexed from, address indexed to, uint256 value) ]"#,
);

async fn get_logs_with_retry(provider: &Provider<Http>, filter: &Filter) -> Result<Vec<Log>> {
    let mut attempts = 0;
    loop {
        match provider.get_logs(filter).await {
            Ok(logs) => return Ok(logs),
            Err(e) => {
                attempts += 1;
                eprintln!("Failed to fetch logs (attempt {}): {:?}. Retrying in 5s...", attempts, e);
                sleep(Duration::from_secs(5)).await;
                if attempts >= 5 {
                    eprintln!("Max retries reached. Skipping this block.");
                    return Ok(vec![]);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let args = Command::from_args();

    // Initialize database
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "indexer.db".to_string());
    let conn = match init_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialize DB: {:?}", e);
            return Ok(());
        }
    };
    println!("DB initialized at {}", db_path);

    // Polygon RPC
    let rpc_url = std::env::var("POLYGON_RPC").unwrap_or_else(|_| "https://polygon-rpc.com/".to_string());
    let provider = match Provider::<Http>::try_from(rpc_url) {
        Ok(p) => p.interval(Duration::from_secs(2)),
        Err(e) => {
            eprintln!("Invalid RPC URL: {:?}", e);
            return Ok(());
        }
    };
    let provider = Arc::new(provider);

    // POL token contract address
    let pol_address: H160 = match "0x1234567890abcdef1234567890abcdef12345678".parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Invalid contract address: {:?}", e);
            return Ok(());
        }
    };
    let pol_contract = PolToken::new(pol_address, provider.clone());

    // Binance addresses
    let binance: Vec<H160> = vec![
        "0xF977814e90dA44bFA03b6295A0616a897441aceC",
        "0xe7804c37c13166fF0b37F5aE0BB07A3aEbb6e245",
        "0x505e71695E9bc45943c58adEC1650577BcA68fD9",
        "0x290275e3db66394C52272398959845170E4DCb88",
        "0xD5C08681719445A5Fdce2Bda98b341A49050d821",
        "0x082489A616aB4D46d1947eE3F912e080815b08DA",
    ]
    .into_iter()
    .filter_map(|s| s.parse::<H160>().ok())
    .collect();

    match args {
        Command::Start => {
            let mut from_block = match provider.get_block_number().await {
                Ok(bn) => bn.as_u64(),
                Err(e) => {
                    eprintln!("Failed to get current block number: {:?}", e);
                    return Ok(());
                }
            };

            loop {
                let to_block = match provider.get_block_number().await {
                    Ok(bn) => bn.as_u64(),
                    Err(e) => {
                        eprintln!("Failed to get latest block number: {:?}. Retrying...", e);
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                if to_block >= from_block {
                    for blk_num in from_block..=to_block {
                        let filter = pol_contract.event::<TransferFilter>()
                            .from_block(blk_num)
                            .to_block(blk_num)
                            .filter;

                        let logs = match get_logs_with_retry(&provider, &filter).await {
                            Ok(l) => l,
                            Err(e) => {
                                eprintln!("Failed to fetch logs for block {}: {:?}", blk_num, e);
                                continue;
                            }
                        };

                        for log in logs {
                            let raw_log = RawLog {
                                topics: log.topics.clone(),
                                data: log.data.to_vec(),
                            };

                            let decoded = match <pol_token::TransferFilter as EthEvent>::decode_log(&raw_log) {
                                Ok(d) => d,
                                Err(e) => {
                                    eprintln!("Failed to decode log: {:?}. Skipping...", e);
                                    continue;
                                }
                            };

                            let timestamp = match provider.get_block(log.block_number.unwrap()).await {
                                Ok(Some(b)) => b.timestamp.as_u64() as i64,
                                Ok(None) => {
                                    eprintln!("Block not found for log: {:?}", log);
                                    continue;
                                }
                                Err(e) => {
                                    eprintln!("Failed to get block for timestamp: {:?}", e);
                                    continue;
                                }
                            };

                            let tx_hash = match log.transaction_hash {
                                Some(hash) => format!("{:?}", hash),
                                None => {
                                    eprintln!("Transaction hash missing for log: {:?}", log);
                                    continue;
                                }
                            };

                            let block_number = log.block_number.unwrap().as_u64() as i64;
                            let from_addr = decoded.from;
                            let to_addr = decoded.to;
                            let value_dec_str = decoded.value.to_string();

                            if let Err(e) = insert_raw_transfer(
                                &conn,
                                &tx_hash,
                                block_number,
                                &format!("{:?}", from_addr),
                                &format!("{:?}", to_addr),
                                &value_dec_str,
                                timestamp,
                            ) {
                                eprintln!("DB insert failed: {:?}", e);
                                continue;
                            }

                            let from_addr_str = format!("{:?}", from_addr).to_lowercase();
                            let to_addr_str = format!("{:?}", to_addr).to_lowercase();
                            let is_in = binance.iter().any(|a| format!("{:?}", a).to_lowercase() == to_addr_str);
                            let is_out = binance.iter().any(|a| format!("{:?}", a).to_lowercase() == from_addr_str);

                            println!("Block {}: In {} Out {} Value {}", block_number, is_in, is_out, value_dec_str);

                            if is_in || is_out {
                                if let Err(e) = update_net_flows_on_transfer(&conn, &value_dec_str, timestamp, is_in, is_out) {
                                    eprintln!("Failed to update net flows: {:?}", e);
                                }
                            }
                        }
                    }
                    from_block = to_block + 1;
                }

                sleep(Duration::from_secs(5)).await;
            }
        }
        Command::Query => {
            match read_net_flows(&conn) {
                Ok((ci, co, net, last)) => {
                    println!("Cumulative In: {}", ci);
                    println!("Cumulative Out: {}", co);
                    println!("Net Flow: {}", net);
                    println!("Last Updated Timestamp: {}", last);
                }
                Err(e) => eprintln!("Failed to read net flows: {:?}", e),
            }
        }
    }

    Ok(())
}
