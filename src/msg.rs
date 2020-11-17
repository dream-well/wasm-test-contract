use crate::state::Gardener;
use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub price: Coin,
    pub number: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    BecomeGardener { name: String },
    BuyBonsai { b_id: u64 },
    SellBonsai { recipient: HumanAddr, b_id: u64 },
    CutBonsai { b_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBonsais {},
    GetGardener { sender: HumanAddr },
    GetGardeners {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AllGardenersResponse {
    pub gardeners: Vec<Gardener>,
}
