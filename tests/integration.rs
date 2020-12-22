//! This integration tes tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//! You can easily convert unit tests to integration tests as follows:
//! 1. Copy them over verbatim
//! 2. Then change
//!      let mut deps = mock_dependencies(20, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)

use cosmwasm_std::{
    attr, coin, coins, from_binary, from_slice, Coin, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, QueryResponse,
};
use cosmwasm_storage::to_length_prefixed;
use cosmwasm_vm::testing::{
    handle, init, mock_env, mock_info, mock_instance, mock_instance_with_balances, query, MockApi,
    MockQuerier, MockStorage,
};
use cosmwasm_vm::{Instance, Storage};
use my_first_contract::msg::{AllGardenersResponse, HandleMsg, InitMsg, QueryMsg};
use my_first_contract::state::{BonsaiList, Gardener, BONSAI_KEY};
use rand::seq::SliceRandom;

const WASM: &[u8] =
    include_bytes!("../target/wasm32-unknown-unknown/release/my_first_contract.wasm");
const BOND_DENOM: &str = "bonsai";

// Create an environment with the given height, sender and funds
fn mock_env_height(height: u64) -> Env {
    let mut env = mock_env();
    env.block.height = height;
    env
}

// this will set up the init for other tests
fn setup_test(
    deps: &mut Instance<MockStorage, MockApi, MockQuerier>,
    env: &Env,
    info: MessageInfo,
    bonsai_price: Coin,
    bonsai_number: u64,
) {
    let init_msg = InitMsg {
        price: bonsai_price,
        number: bonsai_number,
    };
    let _res: InitResponse = init(deps, env.clone(), info, init_msg).unwrap();
}

// return a random bonsai id
fn get_random_bonsai_id(deps: &mut Instance<MockStorage, MockApi, MockQuerier>) -> u64 {
    let result = query(deps, mock_env(), QueryMsg::GetBonsais {}).unwrap();

    let bonsais: BonsaiList = from_binary(&result).unwrap();
    let rand_bonsai = bonsais.bonsais.choose(&mut rand::thread_rng()).unwrap();

    rand_bonsai.id
}

fn become_gardener(
    name: String,
    info: MessageInfo,
    env: Env,
    deps: &mut Instance<MockStorage, MockApi, MockQuerier>,
) -> HandleResponse {
    let msg = HandleMsg::BecomeGardener { name };
    let res: HandleResponse = handle(deps, env.clone(), info, msg).unwrap();
    res
}

fn buy_bonsai(
    bonsai_id: u64,
    info: MessageInfo,
    env: Env,
    deps: &mut Instance<MockStorage, MockApi, MockQuerier>,
) -> HandleResponse {
    let msg = HandleMsg::BuyBonsai { b_id: bonsai_id };
    let res: HandleResponse = handle(deps, env, info, msg).unwrap();
    res
}

fn query_gardener(
    deps: &mut Instance<MockStorage, MockApi, MockQuerier>,
    env: Env,
    addr: HumanAddr,
) -> Gardener {
    // check if the gardeners was saved
    let query_res = query(deps, env.clone(), QueryMsg::GetGardener { sender: addr }).unwrap();
    let gardener: Gardener = from_binary(&query_res).unwrap();
    gardener
}

#[test]
fn test_init() {
    let mut deps = mock_instance(WASM, &[]);

    // Init an empty contract
    let init_msg = InitMsg {
        price: coin(20, BOND_DENOM),
        number: 20,
    };
    let env = mock_env_height(100);
    let info = mock_info("sender", &coins(1000, BOND_DENOM));
    let _res: InitResponse = init(&mut deps, env, info, init_msg).unwrap();

    deps.with_storage(|storage| {
        let key = to_length_prefixed(BONSAI_KEY);
        let data = storage.get(&key).0.unwrap().unwrap();
        let bonsai_list: BonsaiList = from_slice(&data).unwrap();

        assert_eq!(bonsai_list.bonsais.len(), 20);
        Ok(())
    })
    .unwrap();
}

#[test]
fn test_become_gardener_works() {
    let mut deps = mock_instance(WASM, &[]);

    let sender_addr = HumanAddr::from("addr0001");
    let bonsai_price = coin(10, BOND_DENOM);
    let env = mock_env_height(100);
    let info = mock_info(&sender_addr, &coins(1000, BOND_DENOM));
    setup_test(&mut deps, &env, info.clone(), bonsai_price.clone(), 10);

    let mut exp_res = HandleResponse::default();
    exp_res.attributes = vec![
        attr("action", "become_gardener"),
        attr("gardener_addr", &sender_addr),
    ];

    let res = become_gardener("leo".to_string(), info.clone(), env.clone(), &mut deps);

    // verify that the result attributes are equals to the expected ones
    assert_eq!(exp_res, res);

    // check if the gardeners was saved
    let gardener = query_gardener(&mut deps, env.clone(), sender_addr.clone());
    assert_eq!("leo", gardener.name)
}

#[test]
fn test_buy_bonsai_works() {
    let sender_addr = HumanAddr::from("addr0001");
    let bonsai_price = coin(10, BOND_DENOM);

    let mut deps =
        mock_instance_with_balances(WASM, &[(&sender_addr.clone(), &coins(1000, BOND_DENOM))]);
    let env = mock_env_height(100);
    let info = mock_info(sender_addr, &coins(15, BOND_DENOM));

    // setup test environment
    setup_test(&mut deps, &env, info.clone(), bonsai_price.clone(), 10);

    let _res = become_gardener("leo".to_string(), info.clone(), env.clone(), &mut deps);

    let bonsai_id = get_random_bonsai_id(&mut deps);

    let mut exp_res = HandleResponse::default();
    exp_res.attributes = vec![
        attr("action", "buy_bonsai"),
        attr("buyer", &info.sender),
        attr("amount", bonsai_price.amount),
    ];
    let res = buy_bonsai(bonsai_id, info.clone(), env.clone(), &mut deps);
    assert_eq!(exp_res, res);

    // check if the gardeners was saved
    let gardener: Gardener = query_gardener(&mut deps, env.clone(), info.sender.clone());
    assert_eq!("leo", gardener.name);
    assert_eq!(bonsai_id, gardener.bonsais[0].id)
}

#[test]
fn test_sell_bonsai_works() {
    let mut deps = mock_instance(WASM, &[]);

    let sender_addr = HumanAddr::from("addr0001");
    let buyer_addr = HumanAddr::from("addr0002");
    let bonsai_price = coin(0, BOND_DENOM);
    let env = mock_env_height(100);
    let info = mock_info(sender_addr.clone(), &coins(1000, BOND_DENOM));

    // setup test environment
    setup_test(&mut deps, &env, info.clone(), bonsai_price.clone(), 10);

    let bonsai_id = get_random_bonsai_id(&mut deps);

    let _res = become_gardener("leo".to_string(), info.clone(), env.clone(), &mut deps);

    let _res = buy_bonsai(bonsai_id.clone(), info.clone(), env.clone(), &mut deps);

    let _res = become_gardener(
        "ricky".to_string(),
        mock_info(buyer_addr.clone(), &coins(1000, BOND_DENOM)),
        env.clone(),
        &mut deps,
    );

    let query_res: QueryResponse =
        query(&mut deps, env.clone(), QueryMsg::GetGardeners {}).unwrap();

    let all_gardeners_result: AllGardenersResponse = from_binary(&query_res).unwrap();
    assert_eq!(2, all_gardeners_result.gardeners.len());

    let msg = HandleMsg::SellBonsai {
        recipient: buyer_addr.clone(),
        b_id: bonsai_id,
    };
    let res: HandleResponse = handle(
        &mut deps,
        env.clone(),
        mock_info(sender_addr.clone(), &coins(1000, BOND_DENOM)),
        msg,
    )
    .unwrap();

    let mut exp_res = HandleResponse::default();
    exp_res.attributes = vec![
        attr("action", "sell_bonsai"),
        attr("from", info.sender.clone()),
        attr("to", buyer_addr.clone()),
    ];

    assert_eq!(exp_res, res);

    let gardener: Gardener = query_gardener(&mut deps, env.clone(), info.sender.clone());
    assert_eq!(0, gardener.bonsais.len())
}

#[test]
fn test_cut_bonsai_works() {
    let mut deps = mock_instance(WASM, &[]);

    let sender_addr = HumanAddr::from("addr0001");
    let bonsai_price = coin(10, BOND_DENOM);
    let env = mock_env_height(100);
    let info = mock_info(sender_addr, &coins(100, BOND_DENOM));

    // setup test environment
    setup_test(&mut deps, &env, info.clone(), bonsai_price.clone(), 10);

    let bonsai_id = get_random_bonsai_id(&mut deps);
    let _res = become_gardener("leo".to_string(), info.clone(), env.clone(), &mut deps);

    let _res = buy_bonsai(bonsai_id, info.clone(), env.clone(), &mut deps);

    let msg = HandleMsg::CutBonsai { b_id: bonsai_id };

    let res: HandleResponse = handle(&mut deps, env.clone(), info.clone(), msg).unwrap();

    let mut exp_res = HandleResponse::default();
    exp_res.attributes = vec![
        attr("action", "cut_bonsai"),
        attr("owner", info.sender.clone()),
        attr("bonsai_id", bonsai_id),
    ];

    assert_eq!(exp_res, res);

    let gardener = query_gardener(&mut deps, env.clone(), info.sender.clone());
    assert_eq!(0, gardener.bonsais.len())
}

/*
#[test]
fn query_bonsais_works() {
    let mut deps = mock_instance(&[]);
    let bonsai_price = coin(10, "bonsai");
    let bonsai_height = 100;
    let env = mock_env_height(bonsai_height);
    setup_test(&mut deps, &env, bonsai_price.clone(), 10);

    let bonsais = query_bonsais(&mut deps).unwrap();

    assert_eq!(10, bonsais.bonsais.len())
}

#[test]
fn query_gardener_works() {
    let mut deps = mock_instance(&[]);
    let sender_addr = HumanAddr::from("addr0001");
    let bonsai_price = coin(10, "bonsai");
    let bonsai_height = 100;

    let env = mock_env_height(bonsai_height);
    setup_test(&mut deps, &env, bonsai_price.clone(), 10);

    let bonsai = Bonsai::new(bonsai_height, bonsai_price);
    let canonical_addr = &deps.api.canonical_address(&sender_addr).unwrap();

    let gardener = Gardener::new("leo".to_string(), canonical_addr.clone(), vec![bonsai]);

    let _ = gardeners_store(&mut deps.storage).save(canonical_addr.as_slice(), &gardener);

    let res = query_gardener(&mut deps, sender_addr.clone()).unwrap().unwrap();

    assert_eq!(gardener, res)
}

#[test]
fn query_all_gardeners_works() {
    let mut deps = mock_instance(&[]);
    let sender_addr = HumanAddr::from("addr0001");
    let bonsai_price = coin(10, "bonsai");
    let bonsai_height = 100;

    let env = mock_env_height(bonsai_height);
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

    let gardeners = vec![gardener2.clone(), gardener.clone()];

    for el in gardeners.clone() {
        let _ = gardeners_store(&mut deps.storage).save(el.address.as_slice(), &el);
    }

    let res = query_all_gardeners(&mut deps);

    equal(gardeners, res.unwrap().gardeners);
}
 */
