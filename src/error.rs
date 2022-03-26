use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Already Listed")]
    AlreadyListed {},

    #[error("Invalid Price")]
    InvalidPrice {},

    #[error("Only UST")]
    OnlyUST {},

    #[error("Expiry in Past")]
    ExpiryInPast {},

    #[error("Expiry Too Short")]
    ExpiryTooShort {},

    #[error("Expiry Too Long")]
    ExpiryTooLong {},

    #[error("Not Listed")]
    NotListed {},

    #[error("Invalid Funds")]
    InvalidFunds {},

    #[error("Expired")]
    Expired {},

    #[error("Bid Too Low")]
    BidTooLow {},
}
// Add any other custom errors you like here.
// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
