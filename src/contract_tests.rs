use crate::contract::{handle, init, query_all_gardeners, query_bonsais, query_gardener};
use crate::msg::{HandleMsg, InitMsg};
use crate::state::{gardeners_store, Bonsai, Gardener};
use assert::equal;
use cosmwasm_std::testing::{mock_dependencies, mock_env, MockQuerier};
use cosmwasm_std::{
    coin, log, Api, BankMsg, Coin, Decimal, Env, Extern, HandleResponse, HumanAddr, Querier,
    Storage, Validator,
};
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
