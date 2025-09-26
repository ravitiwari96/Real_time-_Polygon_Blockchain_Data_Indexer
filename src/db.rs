use rusqlite::{Connection, params};
use std::path::Path;
use anyhow::{Result, anyhow};
use ethers::types::U256;

pub fn init_db<P: AsRef<Path>>(db_path: P) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    let schema = std::fs::read_to_string("schema.sql")
        .map_err(|e| anyhow!("failed to read schema.sql: {}", e))?;
    conn.execute_batch(&schema)?;
    Ok(conn)
}

pub fn insert_raw_transfer(
    conn: &Connection,
    tx_hash: &str,
    block_number: i64,
    from_addr: &str,
    to_addr: &str,
    value_dec_str: &str,
    timestamp: i64,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO raw_transfers (tx_hash, block_number, from_address, to_address, value, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![tx_hash, block_number, from_addr, to_addr, value_dec_str, timestamp],
    )?;
    Ok(())
}

pub fn read_net_flows(conn: &Connection) -> Result<(String, String, String, i64)> {
    let mut stmt = conn.prepare("SELECT cumulative_in, cumulative_out, net_flow, last_updated FROM net_flows WHERE id = 1")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let ci: String = row.get(0)?;
        let co: String = row.get(1)?;
        let net: String = row.get(2)?;
        let last: i64 = row.get(3)?;
        Ok((ci, co, net, last))
    } else {
        Err(anyhow!("net_flows row not found"))
    }
}

pub fn update_net_flows_on_transfer(conn: &Connection, amount: &str, timestamp: i64, is_in: bool, is_out: bool) -> Result<()> {
    let (cur_in_s, cur_out_s, _cur_net_s, _last) = read_net_flows(conn)?;
    let cur_in = U256::from_dec_str(&cur_in_s).unwrap_or(U256::zero());
    let cur_out = U256::from_dec_str(&cur_out_s).unwrap_or(U256::zero());
    let amt = U256::from_dec_str(amount).unwrap_or(U256::zero());

    let new_in = if is_in { cur_in + amt } else { cur_in };
    let new_out = if is_out { cur_out + amt } else { cur_out };
    let new_net = if new_in >= new_out { new_in - new_out } else { U256::zero() };

    conn.execute(
        "UPDATE net_flows SET cumulative_in = ?1, cumulative_out = ?2, net_flow = ?3, last_updated = ?4 WHERE id = 1",
        params![new_in.to_string(), new_out.to_string(), new_net.to_string(), timestamp],
    )?;
    Ok(())
}
