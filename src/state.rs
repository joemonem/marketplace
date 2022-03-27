use cw721::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Api, CanonicalAddr, Coin, Querier, Storage, Uint128};
use cw_storage_plus::{Item, Map};

pub const OPERATORS: Map<(&[u8], &[u8]), Expiration> = Map::new("operators");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct NFT {
    pub owner: CanonicalAddr,
    pub token_id: String,
    pub price: Coin,
    pub expiry: u64,
    pub contract_addr: CanonicalAddr,
}

pub const NFTLIST: Map<String, NFT> = Map::new("lists");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: CanonicalAddr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryOfferingsResult {
    pub id: String,
    pub token_id: String,
    pub list_price: Coin,
    pub contract_addr: String,
    pub seller: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OfferingsResponse {
    pub offerings: Vec<QueryOfferingsResult>,
}
