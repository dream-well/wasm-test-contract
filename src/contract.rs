use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    bonsai_store, bonsai_store_readonly, gardeners_store, gardeners_store_readonly, BonsaiList,
    Gardener,
};

// version info for migration purposes
// TODO check how this should be added
// const CONTRACT_NAME: &str = "crates.io:bonsai-cw-bragaz";
// const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// init is like the genesis of cosmos SDK
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    // set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let bonsai_list = BonsaiList::grow_bonsais(msg.number, env.block.height, msg.price);
    bonsai_store(&mut deps.storage).save(&bonsai_list)?;

    let mut res = InitResponse::default();
    res.log = vec![
        log("action", "grown_bonsais"),
    ];

    Ok(res)
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::BecomeGardener { name } => handle_become_gardener(deps, env, name),
        HandleMsg::BuyBonsai { b_id } => handle_buy_bonsai(deps, env, b_id),
        HandleMsg::SellBonsai { recipient, b_id } => handle_sell_bonsai(deps, env, recipient, b_id),
        HandleMsg::CutBonsai { b_id } => handle_cut_bonsai(deps, env, b_id),
    }
}

pub fn handle_become_gardener<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
) -> StdResult<HandleResponse> {
    let canonical_addr = &deps.api.canonical_address(&env.message.sender)?;
    let res = gardeners_store(&mut deps.storage).load(canonical_addr.as_slice());
    let gardener = match res {
        Ok(_) => {
            return Err(StdError::generic_err(
                "A gardener with the sender address already exist",
            ))
        }
        Err(_) => Gardener::new(name, canonical_addr.clone(), vec![]),
    };

    gardeners_store(&mut deps.storage).save(canonical_addr.as_slice(), &gardener)?;

    let mut res = HandleResponse::default();
    res.log = vec![
        log("action", "become_gardener"),
        log("gardener_addr", env.message.sender),
    ];

    Ok(res)
}

pub fn handle_buy_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    // try to load bonsai list if present otherwise returns error
    let bonsai_list = bonsai_store(&mut deps.storage).load()?;

    let bonsai = bonsai_list.bonsais.iter().find(|bonsai| bonsai.id == id);

    let bonsai = match bonsai {
        Some(bonsai) => bonsai,
        None => return Err(StdError::not_found("Bonsai not found")),
    };

    // check if the gardener has enough funds to buy the bonsai
    let denom = deps.querier.query_bonded_denom()?;
    let balance = deps
        .querier
        .query_balance(&env.message.sender, &denom.as_str())?;
    if balance.amount < bonsai.price.amount {
        return Err(StdError::generic_err(
            "Insufficient funds to buy the bonsai",
        ));
    }

    // remove the bought bonsai from the garden
    bonsai_store(&mut deps.storage).update(|mut bonsai_list| {
        bonsai_list.bonsais.retain(|bonsai| bonsai.id == id);
        Ok(bonsai_list)
    })?;

    let canonical_addr = &deps.api.canonical_address(&env.message.sender)?;
    // todo check if it's possible to use may_update
    gardeners_store(&mut deps.storage).load(canonical_addr.as_slice())?;
    gardeners_store(&mut deps.storage).update(canonical_addr.as_slice(), |gardener| {
        let mut unwrapped = gardener.unwrap();
        unwrapped.bonsais.push(bonsai.clone());
        Ok(unwrapped)
    })?;

    let res = HandleResponse {
        messages: vec![BankMsg::Send {
            from_address: env.message.sender.clone(),
            to_address: env.contract.address,
            amount: vec![bonsai.price.clone()],
        }
        .into()],
        log: vec![
            log("action", "buy_bonsai"),
            log("buyer", env.message.sender),
            log("amount", bonsai.price.amount),
        ],
        data: None,
    };

    Ok(res)
}

pub fn handle_sell_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    buyer: HumanAddr,
    id: String,
) -> StdResult<HandleResponse> {
    // convert human_addr to canonical
    let seller_addr = &deps.api.canonical_address(&env.message.sender)?;
    // load the store and get the seller
    let mut seller_bonsai_list = gardeners_store(&mut deps.storage)
        .load(seller_addr.as_slice())?
        .bonsais;

    // extract the bonsai to sell
    let bonsai_to_sell = seller_bonsai_list
        .iter()
        .find(|bonsai| bonsai.id == id)
        .clone()
        .ok_or_else(|| StdError::generic_err(format!("No bonsai with {} id found", id)))?
        .clone();

    // check buyer's funds
    let denom = &deps.querier.query_bonded_denom()?;
    let balance = deps.querier.query_balance(&buyer, &denom.as_str())?;

    if balance.amount < bonsai_to_sell.price.amount {
        return Err(StdError::generic_err("Insufficient buyers funds"));
    }

    // add sold bonsai to buyer list
    let buyer_addr = &deps.api.canonical_address(&buyer)?;
    gardeners_store(&mut deps.storage).update(buyer_addr.as_slice(), |gardener| {
        let mut unwrapped = gardener.unwrap();
        unwrapped.bonsais.push(bonsai_to_sell);
        Ok(unwrapped)
    })?;

    // remove the sold bonsai from seller list
    seller_bonsai_list.retain(|b| b.id == id);
    gardeners_store(&mut deps.storage).update(seller_addr.as_slice(), |seller_gardener| {
        let mut unwrapped = seller_gardener.unwrap();
        unwrapped.bonsais = seller_bonsai_list;
        Ok(unwrapped)
    })?;

    let mut res = HandleResponse::default();
    res.log = vec![
        log("action", "sold_bonsai"),
        log("from", env.message.sender),
        log("to", buyer),
    ];

    Ok(res)
}

pub fn handle_cut_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    let owner_addr = deps.api.canonical_address(&env.message.sender)?;
    gardeners_store(&mut deps.storage).update(owner_addr.as_slice(), |gardener| {
        let mut unwrapped = gardener.unwrap();
        unwrapped.bonsais.retain(|b| b.id == id);
        Ok(unwrapped)
    })?;

    let mut res = HandleResponse::default();
    res.log = vec![
        log("action", "cut_bonsai"),
        log("owner", env.message.sender),
        log("bonsai_id", id),
    ];

    Ok(res)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBonsais {} => to_binary(&query_bonsais(deps)?),
        QueryMsg::GetGardeners { sender } => to_binary(&query_gardeners(deps, sender)?),
    }
}

fn query_bonsais<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<BonsaiList> {
    let bonsais = bonsai_store_readonly(&deps.storage).load()?;
    Ok(bonsais)
}

fn query_gardeners<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    sender: HumanAddr,
) -> StdResult<Gardener> {
    let canonical_addr = deps.api.canonical_address(&sender)?;
    let response = gardeners_store_readonly(&deps.storage).may_load(canonical_addr.as_slice());

    match response {
        Ok(response) => Ok(response.unwrap()),
        Err(_) => Err(StdError::not_found(
            "No gardener found with the given address",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coin, Coin};

    const CANONICAL_LENGTH: usize = 20;

    fn mock_env_height<U: Into<HumanAddr>>(sender: U, sent: &[Coin], height: u64) -> Env {
        let mut env = mock_env(sender, sent);
        env.block.height = height;
        env
    }

    // this will set up the init for other tests
    fn setup_test<S: Storage, A: Api, Q: Querier>(
        mut deps: &mut Extern<S, A, Q>,
        env: &Env,
        bonsai_price: Coin,
        bonsai_number: u32,
    ) {
        let init_msg = InitMsg { price: bonsai_price, number: bonsai_number };
        init(deps, env.clone(), init_msg).unwrap();
    }

    #[test]
    fn test_init() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // Init an empty contract
        let init_msg = InitMsg { price: coin(20, "bonsai"), number: 20 };
        let env = mock_env_height("anyone", &[], 100);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let exp_log = vec![
            log("action", "grown_bonsais"),
        ];

        assert_eq!(res.log, exp_log)
    }
}
