use cosmwasm_std::{CanonicalAddr, StdError};
use thiserror::Error;

// thiserror implements Display and ToString if you
// set the `#[error("…")]` attribute for all cases
#[derive(Error, Debug)]
pub enum MyCustomError {
    #[error("{0}")]
    // let thiserror implement From<StdError> for you
    Std(#[from] StdError),
    // this is whatever we want
    #[error("Permission denied: the sender is not the current owner")]
    NotCurrentOwner {
        expected: CanonicalAddr,
        actual: CanonicalAddr,
    },
    #[error("Messages empty. Must reflect at least one message")]
    MessagesEmpty,
}
