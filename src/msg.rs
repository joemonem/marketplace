use cosmwasm_std::{CanonicalAddr, Coin, Uint128};
use cw721::{Approval, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::NFT;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    List {
        expires: u64,
        price: Coin,
        minimum_bid: Uint128,
        token_id: String,
    },
    Buy {
        token_id: String,
    },
    Unlist {
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetListing { token_id: String },
    GetAllListings { start: Uint128, end: Uint128 },
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct QueryListingsResponse {
    pub id: String,
    pub token_id: String,
    pub list_price: Coin,
    pub contract_addr: String,
    pub seller: String,
}
// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListingResponse {
    pub owner: CanonicalAddr,
    pub token_id: String,
    pub price: Coin,
    pub expiry: u64,
    pub approvals: Vec<Approval>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListingsResponse {
    pub listings: Vec<QueryListingsResponse>,
}
