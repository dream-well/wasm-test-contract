use cosmwasm_std::{
    attr, to_binary, BankMsg, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdError, StdResult,
};

use crate::errors::MyCustomError;
use crate::msg::{AllGardenersResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    bonsai_store, bonsai_store_read, gardeners_store, gardeners_store_read, BonsaiList, Gardener,
};

// version info for migration purposes
// TODO check how this should be added
// const CONTRACT_NAME: &str = "crates.io:bonsai-cw-bragaz";
// const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// init is like the genesis of cosmos SDK
pub fn init(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, MyCustomError> {
    // set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let bonsai_list = BonsaiList::grow_bonsais(msg.number, env.block.height, msg.price);
    bonsai_store(deps.storage).save(&bonsai_list)?;
    let mut res = InitResponse::default();
    res.attributes = vec![attr("action", "grown_bonsais")];
    Ok(res)
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, MyCustomError> {
    match msg {
        HandleMsg::BecomeGardener { name } => handle_become_gardener(deps, info, name),
        HandleMsg::BuyBonsai { b_id } => handle_buy_bonsai(deps, env, info, b_id),
        HandleMsg::SellBonsai { recipient, b_id } => {
            handle_sell_bonsai(deps, info, recipient, b_id)
        }
        HandleMsg::CutBonsai { b_id } => handle_cut_bonsai(deps, info, b_id),
    }
}

pub fn handle_become_gardener(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
) -> Result<HandleResponse, MyCustomError> {
    let canonical_addr = &deps.api.canonical_address(&info.sender)?;
    let res = gardeners_store(deps.storage).load(canonical_addr.as_slice());
    let gardener = match res {
        Ok(_) => {
            return Err(MyCustomError::Std(StdError::generic_err(
                "A gardener with the sender address already exist",
            )));
        }
        Err(_) => Gardener::new(name, canonical_addr.clone(), vec![]),
    };

    gardeners_store(deps.storage).save(canonical_addr.as_slice(), &gardener)?;

    let mut res = HandleResponse::default();
    res.attributes = vec![
        attr("action", "become_gardener"),
        attr("gardener_addr", info.sender),
    ];

    Ok(res)
}

fn remove_bonsai(deps: DepsMut, address: CanonicalAddr, bonsai_id: u64) {
    let _ = gardeners_store(deps.storage).update::<_, StdError>(
        address.as_slice(),
        |seller_gardener| {
            let mut unwrapped = seller_gardener.unwrap();
            unwrapped.bonsais.retain(|b| b.id != bonsai_id);
            Ok(unwrapped)
        },
    );
}

pub fn handle_buy_bonsai(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<HandleResponse, MyCustomError> {
    // try to load bonsai list if present otherwise returns error
    let bonsai_list = bonsai_store(deps.storage).load()?;

    let bonsai = bonsai_list.bonsais.iter().find(|bonsai| bonsai.id == id);

    let bonsai = match bonsai {
        Some(bonsai) => bonsai,
        None => return Err(MyCustomError::Std(StdError::not_found("Bonsai not found"))),
    };

    // check if the gardener has enough funds to buy the bonsai
    let sent_funds = info.sent_funds.first().ok_or_else(|| {
        MyCustomError::Std(StdError::generic_err("No funds to complete the purchase"))
    })?;
    if sent_funds.denom == bonsai.price.denom {
        if sent_funds.amount < bonsai.price.amount {
            return Err(MyCustomError::Std(StdError::generic_err(
                "Insufficient funds to buy the bonsai",
            )));
        }
    } else {
        return Err(MyCustomError::Std(StdError::generic_err(
            "Insufficient funds to buy the bonsai",
        )));
    }

    // remove the bought bonsai from the garden
    bonsai_store(deps.storage).update::<_, StdError>(|mut bonsai_list| {
        bonsai_list.bonsais.retain(|bonsai| bonsai.id != id);
        Ok(bonsai_list)
    })?;

    let canonical_addr = &deps.api.canonical_address(&info.sender)?;
    // todo check if it's possible to use may_update
    gardeners_store(deps.storage).update::<_, StdError>(canonical_addr.as_slice(), |gardener| {
        let mut unwrapped = gardener.unwrap();
        unwrapped.bonsais.push(bonsai.clone());
        Ok(unwrapped)
    })?;

    let res = HandleResponse {
        messages: vec![BankMsg::Send {
            from_address: info.sender.clone(),
            to_address: env.contract.address,
            amount: vec![bonsai.price.clone()],
        }
        .into()],
        attributes: vec![
            attr("action", "buy_bonsai"),
            attr("buyer", info.sender),
            attr("amount", bonsai.price.amount),
        ],
        data: None,
    };

    Ok(res)
}

pub fn handle_sell_bonsai(
    deps: DepsMut,
    info: MessageInfo,
    buyer: HumanAddr,
    id: u64,
) -> Result<HandleResponse, MyCustomError> {
    // convert human_addr to canonical
    let seller_addr = &deps.api.canonical_address(&info.sender)?;

    // extract the bonsai to sell
    let bonsai_to_sell = gardeners_store(deps.storage)
        .load(seller_addr.as_slice())?
        .bonsais
        .iter()
        .find(|bonsai| bonsai.id == id)
        .clone()
        .ok_or_else(|| StdError::generic_err(format!("No bonsai with {} id found", id)))?
        .clone();

    // check buyer's funds
    let denom = &deps.querier.query_bonded_denom()?;
    let balance = deps.querier.query_balance(&buyer, &denom.as_str())?;

    if balance.amount < bonsai_to_sell.price.amount {
        return Err(MyCustomError::Std(StdError::generic_err(
            "Insufficient buyers funds",
        )));
    }

    // add sold bonsai to buyer list
    let buyer_addr = &deps.api.canonical_address(&buyer)?;
    gardeners_store(deps.storage).update::<_, StdError>(buyer_addr.as_slice(), |gardener| {
        let mut unwrapped = gardener.unwrap();
        unwrapped.bonsais.push(bonsai_to_sell);
        Ok(unwrapped)
    })?;

    // remove the sold bonsai from seller list
    remove_bonsai(deps, seller_addr.clone(), id);

    let mut res = HandleResponse::default();
    res.attributes = vec![
        attr("action", "sell_bonsai"),
        attr("from", info.sender),
        attr("to", buyer),
    ];

    Ok(res)
}

pub fn handle_cut_bonsai(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<HandleResponse, MyCustomError> {
    let owner_addr = deps.api.canonical_address(&info.sender)?;
    remove_bonsai(deps, owner_addr, id);

    let mut res = HandleResponse::default();
    res.attributes = vec![
        attr("action", "cut_bonsai"),
        attr("owner", info.sender),
        attr("bonsai_id", id),
    ];

    Ok(res)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBonsais {} => to_binary(&query_bonsais(deps)?),
        QueryMsg::GetGardener { sender } => to_binary(&query_gardener(deps, sender)?),
        QueryMsg::GetGardeners {} => to_binary(&query_all_gardeners(deps)?),
    }
}

pub fn query_bonsais(deps: Deps) -> StdResult<BonsaiList> {
    let bonsais = bonsai_store_read(deps.storage).load()?;
    Ok(bonsais)
}

pub fn query_gardener(deps: Deps, sender: HumanAddr) -> StdResult<Option<Gardener>> {
    let canonical_addr = deps.api.canonical_address(&sender)?;
    let response = gardeners_store_read(deps.storage).may_load(canonical_addr.as_slice())?;

    Ok(response)
}

pub fn query_all_gardeners(deps: Deps) -> StdResult<AllGardenersResponse> {
    let res: StdResult<Vec<Gardener>> = gardeners_store_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|item| item.map(|(_k, gardener)| gardener))
        .collect();

    Ok(AllGardenersResponse { gardeners: res? })
}
