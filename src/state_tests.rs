use crate::state::{Bonsai, Gardener};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{coin, Api, HumanAddr};

#[test]
fn new_bonsai() {
    let mut exp_bonsai = Bonsai {
        id: 1,
        birth_date: 100,
        price: coin(145, "testCoin"),
    };

    let cur_bonsai = Bonsai::new(1, 100, exp_bonsai.price.clone());

    exp_bonsai.id = cur_bonsai.id.clone();

    assert_eq!(exp_bonsai, cur_bonsai)
}

#[test]
fn new_gardener() {
    let api = MockApi::default();

    let exp_gardener = Gardener {
        name: "leo".to_string(),
        address: api.canonical_address(&HumanAddr::from("addr")).unwrap(),
        bonsais: vec![],
    };

    let cur_gardener = Gardener::new(
        exp_gardener.name.clone(),
        exp_gardener.address.clone(),
        vec![],
    );

    assert_eq!(exp_gardener, cur_gardener)
}

#[test]
fn grow_bonsais() {
    //let bonsai_list = BonsaiList::grow_bonsais(20, 100, coin(20, "bonsai"), mock_dependencies(&[]););
    //assert_eq!(20, bonsai_list.bonsais.len())
}
