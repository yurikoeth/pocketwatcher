//! Crypto wallet balances via keyless public APIs, priced with CoinGecko
//! (native coins) and Blockscout / DexScreener (tokens). Rate-limited but
//! fine for a handful of personal wallets synced on demand.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use crate::db::Holding;

/// (chain id, CoinGecko id, ticker, decimals)
pub const CHAINS: &[(&str, &str, &str, u32)] = &[
    ("solana", "solana", "SOL", 9),
    ("ethereum", "ethereum", "ETH", 18),
    ("bitcoin", "bitcoin", "BTC", 8),
];

pub fn chain_info(chain: &str) -> Option<&'static (&'static str, &'static str, &'static str, u32)> {
    CHAINS.iter().find(|c| c.0 == chain)
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub id: i64,
    pub name: String,
    pub chain: String,
    pub symbol: String,
    /// Native-coin quantity.
    pub units: f64,
    /// Native-coin price in `currency`.
    pub price: f64,
    /// Total wallet value (native + tokens) in `currency`.
    pub balance: f64,
    pub currency: String,
    pub holdings: Vec<Holding>,
    pub error: Option<String>,
}

pub struct Prices {
    /// CoinGecko id -> price in the requested currency.
    pub by_id: HashMap<String, f64>,
    /// Currency actually used (upper-case).
    pub currency: String,
    /// Multiply a USD price by this to get `currency`.
    pub usd_ratio: f64,
}

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("fincompass/0.1")
        .build()
        .map_err(|e| e.to_string())
}

async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value, String> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("{url}: {e}"))?
        .json()
        .await
        .map_err(|e| format!("{url}: bad JSON: {e}"))
}

async fn rpc(client: &reqwest::Client, url: &str, method: &str, params: Value) -> Result<Value, String> {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let v: Value = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("{url}: {e}"))?
        .json()
        .await
        .map_err(|e| format!("{url}: bad JSON: {e}"))?;
    if let Some(err) = v.get("error") {
        let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("rpc error");
        return Err(format!("{url}: {msg}"));
    }
    v.get("result").cloned().ok_or_else(|| format!("{url}: no result"))
}

/// Try each endpoint in turn; return the first success or the last error.
async fn first_ok<T, F, Fut>(endpoints: &[&'static str], f: F) -> Result<T, String>
where
    F: Fn(&'static str) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut last = String::from("no endpoints");
    for &ep in endpoints {
        match f(ep).await {
            Ok(v) => return Ok(v),
            Err(e) => last = e,
        }
    }
    Err(last)
}

const SOLANA_RPCS: &[&str] = &["https://api.mainnet-beta.solana.com", "https://solana-rpc.publicnode.com"];
const ETH_RPCS: &[&str] = &["https://eth.drpc.org", "https://cloudflare-eth.com", "https://ethereum-rpc.publicnode.com", "https://eth.llamarpc.com"];
const BTC_APIS: &[&str] = &["https://blockstream.info/api/address/", "https://mempool.space/api/address/"];

/// Native-token quantity held at `address` (SOL, ETH or BTC).
pub async fn fetch_units(client: &reqwest::Client, chain: &str, address: &str) -> Result<f64, String> {
    let address = address.trim();
    if address.is_empty() {
        return Err("empty address".into());
    }
    match chain {
        "solana" => {
            first_ok(SOLANA_RPCS, |url| async move {
                let r = rpc(client, url, "getBalance", json!([address])).await?;
                let lamports = r.get("value").and_then(|v| v.as_u64()).ok_or_else(|| format!("{url}: unexpected getBalance shape"))?;
                Ok(lamports as f64 / 1e9)
            })
            .await
        }
        "ethereum" => {
            first_ok(ETH_RPCS, |url| async move {
                let r = rpc(client, url, "eth_getBalance", json!([address, "latest"])).await?;
                let hex = r.as_str().ok_or_else(|| format!("{url}: unexpected eth_getBalance shape"))?;
                let wei = u128::from_str_radix(hex.trim_start_matches("0x"), 16).map_err(|e| format!("{url}: bad hex balance: {e}"))?;
                Ok(wei as f64 / 1e18)
            })
            .await
        }
        "bitcoin" => {
            first_ok(BTC_APIS, |base| async move {
                let v = get_json(client, &format!("{base}{address}")).await?;
                let sum = |scope: &str, key: &str| v.pointer(&format!("/{scope}/{key}")).and_then(|x| x.as_u64()).unwrap_or(0) as i128;
                let sats = sum("chain_stats", "funded_txo_sum") - sum("chain_stats", "spent_txo_sum") + sum("mempool_stats", "funded_txo_sum") - sum("mempool_stats", "spent_txo_sum");
                Ok(sats.max(0) as f64 / 1e8)
            })
            .await
        }
        other => Err(format!("unsupported chain '{other}'")),
    }
}

/// Parse a decimal string (or number) into f64.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Convert a raw integer amount string with `decimals` into units.
fn scale(raw: &str, decimals: u32) -> Option<f64> {
    let raw: u128 = raw.trim().parse().ok()?;
    Some(raw as f64 / 10f64.powi(decimals as i32))
}

fn short(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}…{}", &addr[..4], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}

/// Non-native token positions (ERC-20 / SPL), priced in USD where a market
/// price exists. Bitcoin has none.
pub async fn fetch_token_holdings(client: &reqwest::Client, chain: &str, address: &str) -> Result<Vec<Holding>, String> {
    match chain {
        "ethereum" => eth_tokens(client, address).await,
        "solana" => sol_tokens(client, address).await,
        _ => Ok(vec![]),
    }
}

async fn eth_tokens(client: &reqwest::Client, address: &str) -> Result<Vec<Holding>, String> {
    let base = format!("https://eth.blockscout.com/api/v2/addresses/{}/token-balances", address.trim());
    let mut out = Vec::new();
    let mut url = base.clone();
    for _ in 0..5 {
        let v = get_json(client, &url).await?;
        let items: Vec<Value> = match &v {
            Value::Array(a) => a.clone(),
            Value::Object(_) => v.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default(),
            _ => vec![],
        };
        for it in &items {
            let t = it.get("token").cloned().unwrap_or(Value::Null);
            if t.get("type").and_then(|x| x.as_str()) != Some("ERC-20") {
                continue;
            }
            let reputation = t.get("reputation").and_then(|x| x.as_str()).unwrap_or("ok");
            if reputation == "scam" || reputation == "spam" {
                continue;
            }
            let decimals = t.get("decimals").and_then(num).unwrap_or(18.0) as u32;
            let Some(units) = it.get("value").and_then(|x| x.as_str()).and_then(|raw| scale(raw, decimals)) else { continue };
            if units <= 0.0 {
                continue;
            }
            let contract = t.get("address_hash").or(t.get("address")).and_then(|x| x.as_str()).unwrap_or("").to_string();
            let symbol = t.get("symbol").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(|| short(&contract));
            let name = t.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let price = t.get("exchange_rate").and_then(num).filter(|p| *p > 0.0);
            out.push(Holding { contract, symbol, name, units, price, value: price.map(|p| p * units).unwrap_or(0.0) });
        }
        // Follow pagination for wallets with more than one page of tokens.
        match v.get("next_page_params").filter(|n| !n.is_null()).and_then(|n| n.as_object()) {
            Some(np) if !np.is_empty() => {
                let qs: Vec<String> = np.iter().map(|(k, v)| format!("{k}={}", v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))).collect();
                url = format!("{base}?{}", qs.join("&"));
            }
            _ => break,
        }
    }
    // Blockscout only quotes larger tokens; ask DexScreener for the rest.
    let unpriced: Vec<String> = out.iter().filter(|h| h.price.is_none()).map(|h| h.contract.clone()).collect();
    if !unpriced.is_empty() {
        let meta = dex_prices(client, "ethereum", &unpriced).await;
        for h in out.iter_mut().filter(|h| h.price.is_none()) {
            if let Some(m) = meta.get(&h.contract.to_lowercase()) {
                if m.price > 0.0 {
                    h.price = Some(m.price);
                    h.value = h.units * m.price;
                    if h.name.is_empty() {
                        h.name = m.name.clone();
                    }
                }
            }
        }
    }
    Ok(out)
}

struct DexMeta {
    symbol: String,
    name: String,
    price: f64,
    liquidity: f64,
}

/// DexScreener spot prices (USD) for token contracts on `chain_slug`
/// ("ethereum" / "solana"), keeping the deepest pair per token. Best-effort:
/// tokens it doesn't know are simply absent from the map.
async fn dex_prices(client: &reqwest::Client, chain_slug: &str, contracts: &[String]) -> HashMap<String, DexMeta> {
    let mut meta: HashMap<String, DexMeta> = HashMap::new();
    let wanted: std::collections::HashSet<String> = contracts.iter().map(|c| c.to_lowercase()).collect();
    for chunk in contracts.chunks(30) {
        let url = format!("https://api.dexscreener.com/tokens/v1/{chain_slug}/{}", chunk.join(","));
        let pairs = match get_json(client, &url).await {
            Ok(Value::Array(p)) => p,
            _ => continue,
        };
        for p in pairs {
            let Some(addr) = p.pointer("/baseToken/address").and_then(|a| a.as_str()) else { continue };
            let key = addr.to_lowercase();
            if !wanted.contains(&key) {
                continue;
            }
            let Some(price) = p.get("priceUsd").and_then(num) else { continue };
            let liquidity = p.pointer("/liquidity/usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let better = meta.get(&key).map(|m| liquidity > m.liquidity).unwrap_or(true);
            if better {
                meta.insert(
                    key,
                    DexMeta {
                        symbol: p.pointer("/baseToken/symbol").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        name: p.pointer("/baseToken/name").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        price,
                        liquidity,
                    },
                );
            }
        }
    }
    meta
}

async fn sol_tokens(client: &reqwest::Client, address: &str) -> Result<Vec<Holding>, String> {
    const PROGRAMS: &[&str] = &["TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"];
    let mut by_mint: HashMap<String, f64> = HashMap::new();
    for program in PROGRAMS {
        let r = first_ok(SOLANA_RPCS, |url| async move {
            rpc(client, url, "getTokenAccountsByOwner", json!([address.trim(), { "programId": program }, { "encoding": "jsonParsed" }])).await
        })
        .await?;
        for acc in r.get("value").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
            let info = acc.pointer("/account/data/parsed/info").cloned().unwrap_or(Value::Null);
            let mint = info.get("mint").and_then(|m| m.as_str()).unwrap_or("").to_string();
            let ui = info.pointer("/tokenAmount/uiAmount").and_then(|x| x.as_f64()).unwrap_or(0.0);
            if !mint.is_empty() && ui > 0.0 {
                *by_mint.entry(mint).or_insert(0.0) += ui;
            }
        }
    }
    if by_mint.is_empty() {
        return Ok(vec![]);
    }
    let mints: Vec<String> = by_mint.keys().cloned().collect();
    let meta = dex_prices(client, "solana", &mints).await;
    let mut out = Vec::new();
    for (mint, units) in by_mint {
        let (symbol, name, price) = match meta.get(&mint.to_lowercase()) {
            Some(m) => (if m.symbol.is_empty() { short(&mint) } else { m.symbol.clone() }, m.name.clone(), Some(m.price).filter(|p| *p > 0.0)),
            None => (short(&mint), String::new(), None),
        };
        out.push(Holding { contract: mint, symbol, name, units, price, value: price.map(|p| p * units).unwrap_or(0.0) });
    }
    Ok(out)
}

/// Native-coin spot prices keyed by CoinGecko id, plus the USD conversion
/// ratio for token prices. Falls back to USD when CoinGecko doesn't quote the
/// requested currency.
pub async fn fetch_prices(client: &reqwest::Client, currency: &str) -> Result<Prices, String> {
    let vs = currency.trim().to_lowercase();
    let ids: Vec<&str> = CHAINS.iter().map(|c| c.1).collect();
    let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={},usd", ids.join(","), vs);
    let v = get_json(client, &url).await.map_err(|e| format!("coingecko: {e}"))?;
    let mut by_id = HashMap::new();
    let mut used = vs.clone();
    let mut usd_ratio = 1.0;
    for id in ids {
        let entry = v.get(id).cloned().unwrap_or(Value::Null);
        let usd = entry.get("usd").and_then(|p| p.as_f64()).ok_or_else(|| format!("coingecko: no price for {id}"))?;
        let p = match entry.get(&vs).and_then(|p| p.as_f64()) {
            Some(p) => {
                if usd > 0.0 {
                    usd_ratio = p / usd;
                }
                p
            }
            None => {
                used = "usd".into();
                usd
            }
        };
        by_id.insert(id.to_string(), p);
    }
    if used == "usd" {
        usd_ratio = 1.0;
    }
    Ok(Prices { by_id, currency: used.to_uppercase(), usd_ratio })
}
