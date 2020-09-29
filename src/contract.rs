use cosmwasm_std::{to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError, StdResult, Storage, Empty, CosmosMsg, HumanAddr, BankMsg, log};

use cw2::set_contract_version;

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{bonsai_store, bonsai_store_readonly, gardeners_store, gardeners_store_readonly,
                   Bonsai, Gardener, BonsaiList};
use std::any::Any;

// version info for migration purposes
const CONTRACT_NAME: &str = "crates.io:bonsai-cw-bragaz";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// init is like the genesis of cosmos SDK
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let bonsai_list = BonsaiList::grow_bonsais(msg.water, msg.number, env.block.height);
    bonsai_store(&mut deps.storage).save(&bonsai_list)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg<Empty>,
) -> StdResult<HandleResponse<Empty>> {
    match msg {
        HandleMsg::BuyBonsai {
            msgs,
            id
        } => handle_buy_bonsai(deps, env, msgs, id),
        HandleMsg::SellBonsai {
            msgs,
            owner,
            recipient,
            id
        } => handle_sell_bonsai(deps, env, msgs, owner, recpient, id),
        HandleMsg::CutBonsai { owner, id } => handle_cut_bonsai(deps, env, owner, id),
        HandleMsg::PourWater { owner, ids } => handle_pour_water(deps, env, owner, ids)
    }
}

pub fn handle_buy_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
    id: String,
) -> StdResult<HandleResponse> {
    // try to load bonsai list if present otherwise returns error
    let mut bonsai_list = bonsai_store(&mut deps.storage).load()?;

    let bonsai = bonsai_list.bonsais
        .iter()
        .find(| &&bonsai | bonsai.id == id )?;

    let balance = deps.querier.query_balance(&env.message.sender, &String::from("shell"))?;
    if balance.amount < bonsai.price {
        return Err(StdError::generic_err("Insufficient balance to buy the bonsai"))
    }

    bonsai_list.bonsais.retain(| &bonsai| bonsai.id == id);

    bonsai_store(&mut deps.storage).save(&bonsai_list)?;

    let res = HandleResponse {
        messages: vec![BankMsg::Send {
            from_address: env.message.sender.clone(),
            to_address: env.contract.address,
            amount: vec![bonsai.price],
        }.into()],
        log: vec![
            log("action", "claim"),
            log("from", env.message.sender),
            log("amount", bonsai.price.clone()),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_sell_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
    owner: HumanAddr,
    recipient: HumanAddr,
    id: String,
) -> StdResult<HandleResponse> {

}

pub fn handle_cut_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    id: String,
) -> StdResult<HandleResponse> {

}

pub fn handle_pour_water<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    id: Vec<String>,
) -> StdResult<HandleResponse> {

}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBonsais { owner } => to_binary(&query_bonsais(deps, owner)?),
        QueryMsg::GetGardeners {} => to_binary(&query_gardeners(deps)?),
    }
}

fn query_bonsais<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, owner: HumanAddr) -> StdResult<BonsaisResponse> {
    Ok(CountResponse { count: state.count })
}

fn query_gardeners<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<GardenersResponse> {

}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // beneficiary can release it
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Increment {};
        let _res = handle(&mut deps, env, msg).unwrap();

        // should increase counter by 1
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // beneficiary can release it
        let unauth_env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Reset { count: 5 };
        let res = handle(&mut deps, unauth_env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_env = mock_env("creator", &coins(2, "token"));
        let msg = HandleMsg::Reset { count: 5 };
        let _res = handle(&mut deps, auth_env, msg).unwrap();

        // should now be 5
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
}
