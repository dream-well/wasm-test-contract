use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Order, Querier, StdError, StdResult, Storage,
};

use crate::msg::{AllGardenersResponse, HandleMsg, InitMsg, QueryMsg};
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
    res.log = vec![log("action", "grown_bonsais")];

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
            ));
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

fn remove_bonsai<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    address: CanonicalAddr,
    bonsai_id: String,
) {
    let _ = gardeners_store(&mut deps.storage).update(address.as_slice(), |seller_gardener| {
        let mut unwrapped = seller_gardener.unwrap();
        unwrapped.bonsais.retain(|b| b.id != bonsai_id);
        Ok(unwrapped)
    });
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
        bonsai_list.bonsais.retain(|bonsai| bonsai.id != id);
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

    // extract the bonsai to sell
    let bonsai_to_sell = gardeners_store(&mut deps.storage)
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
    remove_bonsai(deps, seller_addr.clone(), id);

    let mut res = HandleResponse::default();
    res.log = vec![
        log("action", "sell_bonsai"),
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
    remove_bonsai(deps, owner_addr.clone(), id.clone());

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
        QueryMsg::GetGardener { sender } => to_binary(&query_gardener(deps, sender)?),
        QueryMsg::GetGardeners {} => to_binary(&query_all_gardeners(deps)),
    }
}

fn query_bonsais<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<BonsaiList> {
    let bonsais = bonsai_store_readonly(&deps.storage).load()?;
    Ok(bonsais)
}

fn query_gardener<S: Storage, A: Api, Q: Querier>(
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

fn query_all_gardeners<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<AllGardenersResponse> {
    let res: StdResult<Vec<Gardener>> = gardeners_store_readonly(&deps.storage)
        .range(None, None, Order::Ascending)
        .map(|item| item.and_then(|(_k, gardener)| Ok(gardener)))
        .collect();

    Ok(AllGardenersResponse { gardeners: res? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Bonsai;
    use assert::equal;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockQuerier};
    use cosmwasm_std::{coin, Coin, Decimal, Validator};
    use rand::seq::SliceRandom;

    const CANONICAL_LENGTH: usize = 20;
    const DEFAULT_VALIDATOR: &str = "default-validator";

    // Mock validator constructor
    fn sample_validator<U: Into<HumanAddr>>(addr: U) -> Validator {
        Validator {
            address: addr.into(),
            commission: Decimal::percent(3),
            max_commission: Decimal::percent(10),
            max_change_rate: Decimal::percent(1),
        }
    }

    // Create an environment with the given height, sender and funds
    fn mock_env_height<U: Into<HumanAddr>>(sender: U, sent: &[Coin], height: u64) -> Env {
        let mut env = mock_env(sender, sent);
        env.block.height = height;
        env
    }

    // Create a mock validator with a given staking
    fn set_validator(querier: &mut MockQuerier, denom: &str) {
        querier.update_staking(denom, &[sample_validator(DEFAULT_VALIDATOR)], &[]);
    }

    // Set the balance for the given address
    fn set_balance(querier: &mut MockQuerier, addr: HumanAddr, balance: Vec<Coin>) {
        querier.update_balance(&addr, balance);
    }

    // this will set up the init for other tests
    fn setup_test<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        env: &Env,
        bonsai_price: Coin,
        bonsai_number: u32,
    ) {
        let init_msg = InitMsg {
            price: bonsai_price,
            number: bonsai_number,
        };
        init(deps, env.clone(), init_msg).unwrap();
    }

    // return a random bonsai id
    fn get_random_bonsai_id<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) -> String {
        let bonsais = query_bonsais(deps).unwrap().bonsais;
        let rand_bonsai = bonsais.choose(&mut rand::thread_rng()).unwrap();

        rand_bonsai.id.clone()
    }

    #[test]
    fn test_init() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // Init an empty contract
        let init_msg = InitMsg {
            price: coin(20, "bonsai"),
            number: 20,
        };
        let env = mock_env_height("anyone", &[], 100);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let exp_log = vec![log("action", "grown_bonsais")];

        assert_eq!(res.log, exp_log)
    }

    #[test]
    fn test_become_gardener_works() {
        let mut deps = mock_dependencies(20, &[]);

        let sender_addr = HumanAddr::from("addr0001");
        let bond_denom = "bonsai";
        let bonsai_price = coin(10, bond_denom);
        let bonsai_height = 100;
        let env = mock_env_height(&sender_addr, &[], bonsai_height);
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);

        let mut exp_res = HandleResponse::default();
        exp_res.log = vec![
            log("action", "become_gardener"),
            log("gardener_addr", &sender_addr),
        ];

        let msg = HandleMsg::BecomeGardener {
            name: String::from("leo"),
        };
        let res = handle(&mut deps, env.clone(), msg);

        // verify it not fails
        assert!(res.is_ok());

        assert_eq!(exp_res, res.unwrap())
    }

    #[test]
    fn test_buy_bonsai_works() {
        let mut deps = mock_dependencies(20, &[]);

        let sender_addr = HumanAddr::from("addr0001");
        let bond_denom = "bonsai";
        let bonsai_price = coin(10, bond_denom);
        let bonsai_height = 100;
        let env = mock_env_height(&sender_addr, &[], bonsai_height);

        // setup test environment
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);
        set_validator(&mut deps.querier, bond_denom);
        set_balance(
            &mut deps.querier,
            env.message.sender.clone(),
            vec![coin(1000, bond_denom)],
        );

        let canonical_addr = &deps.api.canonical_address(&sender_addr).unwrap();
        let gardener = Gardener::new("leo".to_string(), canonical_addr.clone(), vec![]);
        let _ = gardeners_store(&mut deps.storage).save(canonical_addr.as_slice(), &gardener);

        let bonsai_id = get_random_bonsai_id(&mut deps);

        let exp_res = HandleResponse {
            messages: vec![BankMsg::Send {
                from_address: env.message.sender.clone(),
                to_address: env.contract.address.clone(),
                amount: vec![bonsai_price.clone()],
            }
            .into()],
            log: vec![
                log("action", "buy_bonsai"),
                log("buyer", &env.message.sender),
                log("amount", bonsai_price.amount),
            ],
            data: None,
        };

        let msg = HandleMsg::BuyBonsai { b_id: bonsai_id };

        let res = handle(&mut deps, env.clone(), msg);

        assert!(res.is_ok());
        assert_eq!(exp_res, res.unwrap())
    }

    #[test]
    fn test_sell_bonsai_works() {
        let mut deps = mock_dependencies(20, &[]);

        let sender_addr = HumanAddr::from("addr0001");
        let buyer_addr = HumanAddr::from("addr0002");
        let bond_denom = "bonsai";
        let bonsai_price = coin(10, bond_denom);
        let bonsai_height = 100;
        let env = mock_env_height(&sender_addr, &[], bonsai_height);

        // setup test environment
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);
        set_validator(&mut deps.querier, bond_denom);
        set_balance(
            &mut deps.querier,
            buyer_addr.clone(),
            vec![coin(1000, bond_denom)],
        );

        let bonsai = query_bonsais(&deps)
            .unwrap()
            .bonsais
            .first()
            .unwrap()
            .clone();

        let canonical_addr = &deps.api.canonical_address(&sender_addr).unwrap();
        let gardener = Gardener::new(
            "leo".to_string(),
            canonical_addr.clone(),
            vec![bonsai.clone()],
        );
        let _ = gardeners_store(&mut deps.storage).save(canonical_addr.as_slice(), &gardener);

        let canonical_buyer_addr = &deps.api.canonical_address(&buyer_addr).unwrap();
        let buyer = Gardener::new("ricky".to_string(), canonical_buyer_addr.clone(), vec![]);
        let _ = gardeners_store(&mut deps.storage).save(canonical_buyer_addr.as_slice(), &buyer);

        let gardeners = query_all_gardeners(&deps).unwrap();
        assert_eq!(2, gardeners.gardeners.len());

        let msg = HandleMsg::SellBonsai {
            recipient: buyer_addr.clone(),
            b_id: bonsai.clone().id,
        };
        let res = handle(&mut deps, env.clone(), msg);

        let mut exp_res = HandleResponse::default();
        exp_res.log = vec![
            log("action", "sell_bonsai"),
            log("from", env.message.sender),
            log("to", buyer_addr.clone()),
        ];

        assert_eq!(exp_res, res.unwrap());

        let gardener = query_gardener(&deps, sender_addr).unwrap();
        assert_eq!(0, gardener.bonsais.len())
    }

    #[test]
    fn test_cut_bonsai_works() {
        let mut deps = mock_dependencies(20, &[]);

        let sender_addr = HumanAddr::from("addr0001");
        let bond_denom = "bonsai";
        let bonsai_price = coin(10, bond_denom);
        let bonsai_height = 100;
        let env = mock_env_height(&sender_addr, &[], bonsai_height);

        // setup test environment
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);

        let canonical_addr = &deps.api.canonical_address(&sender_addr).unwrap();
        let bonsai = Bonsai::new(bonsai_height, bonsai_price);
        let gardener = Gardener::new(
            "leo".to_string(),
            canonical_addr.clone(),
            vec![bonsai.clone()],
        );

        let _ = gardeners_store(&mut deps.storage).save(canonical_addr.as_slice(), &gardener);

        let msg = HandleMsg::CutBonsai {
            b_id: bonsai.id.clone(),
        };

        let res = handle(&mut deps, env.clone(), msg);
        let mut exp_res = HandleResponse::default();
        exp_res.log = vec![
            log("action", "cut_bonsai"),
            log("owner", env.message.sender.clone()),
            log("bonsai_id", bonsai.id.clone()),
        ];

        assert!(res.is_ok());
        assert_eq!(exp_res, res.unwrap());

        let gardener = query_gardener(&deps, env.message.sender.clone()).unwrap();

        assert_eq!(0, gardener.bonsais.len())
    }

    #[test]
    fn query_bonsais_works() {
        let mut deps = mock_dependencies(20, &[]);

        let sender_addr = HumanAddr::from("addr0001");
        let bonsai_price = coin(10, "bonsai");
        let bonsai_height = 100;
        let env = mock_env_height(sender_addr, &[], bonsai_height);
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);

        let bonsais = query_bonsais(&deps).unwrap();

        assert_eq!(10, bonsais.bonsais.len())
    }

    #[test]
    fn query_gardener_works() {
        let mut deps = mock_dependencies(20, &[]);
        let sender_addr = HumanAddr::from("addr0001");
        let bonsai_price = coin(10, "bonsai");
        let bonsai_height = 100;

        let env = mock_env_height(&sender_addr, &[], bonsai_height);
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);

        let bonsai = Bonsai::new(bonsai_height, bonsai_price);
        let canonical_addr = &deps.api.canonical_address(&sender_addr).unwrap();

        let gardener = Gardener::new("leo".to_string(), canonical_addr.clone(), vec![bonsai]);

        let _ = gardeners_store(&mut deps.storage).save(canonical_addr.as_slice(), &gardener);

        let res = query_gardener(&deps, sender_addr.clone());

        assert_eq!(gardener, res.unwrap())
    }

    #[test]
    fn query_all_gardeners_works() {
        let mut deps = mock_dependencies(20, &[]);
        let sender_addr = HumanAddr::from("addr0001");
        let bonsai_price = coin(10, "bonsai");
        let bonsai_height = 100;

        let env = mock_env_height(&sender_addr, &[], bonsai_height);
        setup_test(&mut deps, &env, bonsai_price.clone(), 10);

        let bonsai = Bonsai::new(bonsai_height, bonsai_price);
        let canonical_addr = &deps.api.canonical_address(&sender_addr).unwrap();
        let other_addr = HumanAddr::from("addr0002");
        let other_addr = &deps.api.canonical_address(&other_addr).unwrap();

        let gardener = Gardener::new(
            "leo".to_string(),
            canonical_addr.clone(),
            vec![bonsai.clone()],
        );

        let gardener2 = Gardener::new(
            "ricky".to_string(),
            other_addr.clone(),
            vec![bonsai.clone()],
        );

        let gardeners = vec![gardener.clone(), gardener2.clone()];

        for el in gardeners.clone() {
            let _ = gardeners_store(&mut deps.storage).save(el.address.as_slice(), &el);
        }

        let res = query_all_gardeners(&deps);

        equal(gardeners, res.unwrap().gardeners);
    }
}
