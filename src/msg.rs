use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr, Empty, CosmosMsg};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub water: bool,
    pub number: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg<T = Empty>
    where
        T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    PourWater { owner: HumanAddr, ids: Vec<String>},
    CutBonsai { owner: HumanAddr, id: String },
    SellBonsai { msgs: Vec<CosmosMsg<T>>, owner: HumanAddr, recipient: HumanAddr, id: String },
    BuyBonsai { msgs: Vec<CosmosMsg<T>>, id: String }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBonsais { owner: HumanAddr },
    GetGardeners { }
}
