use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Coin, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub static BONSAI_KEY: &[u8] = b"bonsai";
pub static GARDENERS_KEY: &[u8] = b"gardener";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bonsai {
    pub id: String,
    // block height at which the bonsai was created
    pub birth_date: u64,
    pub price: Coin,
}

impl Bonsai {
    // not a method but an associate function
    pub fn new(birth_date: u64, price: Coin) -> Bonsai {
        let id: String = thread_rng().sample_iter(&Alphanumeric).take(8).collect();

        Bonsai {
            id,
            birth_date,
            price,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct BonsaiList {
    pub bonsais: Vec<Bonsai>,
}

impl BonsaiList {
    /// grow some bonsais from a given number, watering each one of those
    pub fn grow_bonsais(number: u32, birth_date: u64, price: Coin) -> BonsaiList {
        let mut i = 0;
        let mut bonsai_list = BonsaiList { bonsais: vec![] };
        while i < number {
            bonsai_list
                .bonsais
                .push(Bonsai::new(birth_date, price.clone()));
            i += 1;
        }
        bonsai_list
    }
}

/// return a writable bonsais list
pub fn bonsai_store<S: Storage>(storage: &mut S) -> Singleton<S, BonsaiList> {
    singleton(storage, BONSAI_KEY)
}

/// return a read-only bonsais list
pub fn bonsai_store_readonly<S: Storage>(storage: &S) -> ReadonlySingleton<S, BonsaiList> {
    singleton_read(storage, BONSAI_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Gardener {
    pub name: String,
    pub address: CanonicalAddr,
    pub bonsais: Vec<Bonsai>,
}

impl Gardener {
    // associate function: constructor
    pub fn new(name: String, address: CanonicalAddr, bonsais: Vec<Bonsai>) -> Gardener {
        Gardener {
            name,
            address,
            bonsais,
        }
    }
}

/// return a writable gardeners' bucket
pub fn gardeners_store<S: Storage>(storage: &mut S) -> Bucket<S, Gardener> {
    bucket(GARDENERS_KEY, storage)
}

/// return a read-only gardeners' bucket
pub fn gardeners_store_readonly<S: Storage>(storage: &S) -> ReadonlyBucket<S, Gardener> {
    bucket_read(GARDENERS_KEY, storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coin, Api, HumanAddr};
    use cosmwasm_std::testing::MockApi;

    #[test]
    fn new_bonsai() {
        let mut exp_bonsai = Bonsai{
            id: "".to_string(),
            birth_date: 100,
            price: coin(145, "testCoin"),
        };

        let cur_bonsai = Bonsai::new(100, exp_bonsai.price.clone());

        exp_bonsai.id = cur_bonsai.id.clone();

        assert_eq!(exp_bonsai, cur_bonsai)
    }

    #[test]
    fn new_gardener() {
        let api = MockApi::new(20);

        let exp_gardener = Gardener{
            name: "leo".to_string(),
            address: api.canonical_address(&HumanAddr::from("addr")).unwrap(),
            bonsais: vec![]
        };

        let cur_gardener = Gardener::new(
            exp_gardener.name.clone(),
            exp_gardener.address.clone(),
            vec![]);

        assert_eq!(exp_gardener, cur_gardener)
    }

    #[test]
    fn grow_bonsais() {
        let bonsai_number = 4;
        let bonsai_list = BonsaiList::grow_bonsais(
            bonsai_number,
            10,
            coin(145, "testCoin")
        );

        assert_eq!(4, bonsai_list.bonsais.len())
    }
}
