use std::str::from_utf8;

#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    coin, entry_point, to_binary, Api, BankMsg, Binary, BlockInfo, Coin, CosmosMsg, Deps, DepsMut,
    Env, MessageInfo, Order, Pair, Querier, QuerierWrapper, QueryRequest, Response, StdError,
    StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw721::{Cw721QueryMsg, Expiration, OwnerOfResponse};
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg, Extension,
    MintMsg,
};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, ListingResponse, ListingsResponse, QueryListingsResponse, QueryMsg,
};
use crate::state::{Approval, OfferingsResponse, QueryOfferingsResult, NFT, NFTLIST, OPERATORS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::List {
            expires,
            price,
            minimum_bid,
            token_id,
        } => execute_list(
            deps,
            env,
            info,
            price,
            expires,
            minimum_bid,
            token_id,
            contract_address,
        ),
        ExecuteMsg::Buy { token_id } => execute_buy(deps, env, info, token_id),
        ExecuteMsg::Unlist { token_id } => todo!(),
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => execute_approve(deps, env, info, spender, token_id, expires),
    }
}
fn get_token_owner(
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    token_id: String,
) -> Result<String, ContractError> {
    let address = CW721_CONTRACT.load(storage)?;
    let res: OwnerOfResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address,
        msg: to_binary(&Cw721QueryMsg::OwnerOf {
            token_id,
            include_expired: None,
        })?,
    }))?;
    Ok(res.owner)
}
// ideally should be Deps since the function doesn't make any changes
// fn is_in_list(token_id: String, deps: DepsMut) -> bool {
//     let presence = NFTLIST.load(deps.storage, token_id);
//     match presence {
//         Ok(NFT {
//             owner,
//             token_id,
//             price,
//             expiry,
//             approvals,
//         }) => return true,
//         _ => return false,
//     }
// }
fn check_can_approve(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token: &NFT,
) -> Result<(), ContractError> {
    // owner can approve
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    if token.owner == sender_raw {
        return Ok(());
    }
    // operator can approve
    let op = OPERATORS.may_load(deps.storage, (&token.owner, &sender_raw))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}
pub fn _update_approvals(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    spender: &String,
    token_id: &str,
    // if add == false, remove. if add == true, remove then set with this expiration
    add: bool,
    expires: Option<Expiration>,
) -> Result<NFT, ContractError> {
    let mut token = NFTLIST.load(deps.storage, (&token_id).to_string())?;
    // ensure we have permissions
    check_can_approve(&deps, env, info, &token)?;

    // update the approval list (remove any for the same spender before adding)
    let spender_raw = deps.api.addr_canonicalize(&spender)?;
    token.approvals = token
        .approvals
        .into_iter()
        .filter(|apr| apr.spender != spender_raw)
        .collect();

    // only difference between approve and revoke
    if add {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(ContractError::Expired {});
        }
        let approval = Approval {
            spender: spender_raw,
            expires,
        };
        token.approvals.push(approval);
    }

    NFTLIST.save(deps.storage, (&token_id).to_string(), &token)?;

    Ok(token)
}
pub fn execute_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    spender: String,
    token_id: String,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    _update_approvals(deps, &env, &info, &spender, &token_id, true, expires)?;

    Ok(Response::new()
        .add_attribute("action", "approve")
        .add_attribute("sender", info.sender)
        .add_attribute("spender", spender)
        .add_attribute("token_id", token_id))
}
/// returns true if the sender can transfer ownership of the token
fn check_can_send(
    deps: &Deps,
    env: &Env,
    info: &MessageInfo,
    token: &NFT,
) -> Result<(), ContractError> {
    // owner can send
    let sender_raw = deps.api.addr_canonicalize(&info.sender.to_string())?;
    if token.owner == sender_raw {
        return Ok(());
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == sender_raw && !apr.expires.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let op = OPERATORS.may_load(deps.storage, (&token.owner, &sender_raw))?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}
pub fn _transfer_nft(
    depsmut: DepsMut,
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    recipient: &String,
    token_id: &str,
) -> Result<NFT, ContractError> {
    let mut token = NFTLIST.load(depsmut.storage, (&token_id).to_string())?;
    // ensure we have permissions
    check_can_send(&deps, env, info, &token)?;
    // set owner and remove existing approvals
    token.owner = depsmut.api.addr_canonicalize(recipient)?;
    token.approvals = vec![];
    NFTLIST.save(depsmut.storage, (&token_id).to_string(), &token)?;
    Ok(token)
}
pub fn execute_list(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // setting price as coin naturally sets the denom
    price: Coin,
    expires: u64,
    minimum_bid: Uint128,
    token_id: String,
    contract_address: String,
) -> Result<Response, ContractError> {
    // get token owner
    let token_owner = get_token_owner(deps.storage, querier, token_id)?;
    // //this should return an error since the token_id shouldn't be already listed
    // let mut list = NFTLIST.load(deps.storage, token_id);

    // has to own the NFT to list it
    if info.sender.to_string() != token_owner {
        Err(ContractError::Unauthorized {});
    }
    //can't already be listed
    if is_in_list(token_id, deps) == true {
        return Err(ContractError::AlreadyListed {});
    }
    // can't set price to 0
    if price.amount <= Uint128::new(0) {
        Err(ContractError::InvalidPrice {});
    }
    // let's say our marketplace only deals with UST
    if price.denom != "UST".to_string() {
        Err(ContractError::OnlyUST {});
    }
    // expiry can't be in the past
    if expires <= env.block.height {
        Err(ContractError::ExpiryInPast {});
    }
    // expiry can't be impossibly short, say less than 10 seconds, for the txn to register, front end to display it
    if expires < env.block.height + 10 {
        Err(ContractError::ExpiryTooShort {});
    }
    // expiry can't be too long, let's say 6 months
    if expires > env.block.height + 15780000 {
        Err(ContractError::ExpiryTooLong {});
    }
    let spender = "contractAddress".to_string();
    let approvals = execute_approve(deps, env, info, spender, token_id, expires)?;
    // add the NFT ID to the list of items for sale
    let newnft = NFT {
        owner: deps.api.addr_canonicalize(&info.sender.to_string())?,
        token_id,
        price,
        expiry: expires,
        contract_addr: deps.api.addr_canonicalize(&contract_address.to_string())?,
    };
    let newlist = NFTLIST.save(deps.storage, token_id, &newnft)?;

    let res = Response::new()
        .add_attribute("action", "list")
        .add_attribute("ID", token_id)
        .add_attribute("expires", expires.to_string())
        .add_attribute("price", price.to_string())
        .add_attribute("minimum_bid", minimum_bid.to_string());

    Ok(res)
}
pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let mut list = NFTLIST.load(deps.storage, token_id)?;
    // extract owner of NFT
    let seller = list.owner;
    //nft should be already be listed, happens automatically with loading the specific key, there's the ? in case it fails
    // check exact price
    if info.funds[0].amount != list.price.amount {
        Err(ContractError::InvalidFunds {});
    }
    //check if item is expired
    if list.expiry <= env.block.height {
        Err(ContractError::Expired {});
    }

    //remove from list
    NFTLIST.remove(deps.storage, token_id);
    _transfer_nft(&depsmut, &deps, &env, &info, &recipient, &token_id);

    // send funds to seller
    // transfer NFT to buyer
    Ok(Response::new()
        // Send funds to the original owner.
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: list.owner.to_string(),
            amount: vec![info.funds[0]],
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_auction_state.token_address.clone(),
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: token_id.clone(),
            })?,
            funds: vec![],
        })))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetListing { token_id } => to_binary(&query_listing(deps, token_id)?),
        QueryMsg::GetAllListings { start: (), end: () } => to_binary(&query_offerings(deps)?),
    }
}

fn query_listing(deps: Deps, token_id: String) -> StdResult<ListingResponse> {
    let state = NFTLIST.load(deps.storage, token_id)?;
    Ok(ListingResponse {
        owner: state.owner,
        token_id,
        price: state.price,
        expiry: state.expiry,
    })
}

// fn query_alllistings(deps: Deps) -> StdResult<ListingsResponse> {
//     let list = NFTLIST.load(deps.storage, token_id[0])?;
//     Ok(ListingsResponse { listings })
// }

#[cfg_attr(not(feature = "library"), entry_point)]
fn query_offerings(deps: Deps) -> StdResult<OfferingsResponse> {
    let res: StdResult<Vec<QueryOfferingsResult>> = NFTLIST
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| parse_offering(kv_item))
        .collect();

    Ok(OfferingsResponse {
        offerings: res?, // Placeholder
    })
}

fn parse_offering(item: StdResult<Pair<NFT>>) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        let id = from_utf8(&k)?;
        Ok(QueryOfferingsResult {
            id: id.to_string(),
            token_id: offering.token_id,
            list_price: offering.price,
            contract_addr: offering.contract_addr.to_string(),
            seller: offering.owner.to_string(),
        })
    })
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary};

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies(&[]);

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(1000, "earth"));

//         // we can just call .unwrap() to assert this was a success
//         let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(17, value.count);
//     }

//     #[test]
//     fn increment() {
//         let mut deps = mock_dependencies(&coins(2, "token"));

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Increment {};
//         let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // should increase counter by 1
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::CountResponse {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(18, value.count);
//     }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies(&coins(2, "token"));

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let unauth_info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
//         match res {
//             Err(ContractError::Unauthorized {}) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_info = mock_info("creator", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//         // should now be 5
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
// }
