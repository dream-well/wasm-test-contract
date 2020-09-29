use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Storage, Coin};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton,
                       bucket, bucket_read, Bucket, ReadonlyBucket};

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub static BONSAI_KEY: &[u8] = b"bonsai";
pub static GARDENERS_KEY: &[u8] = b"gardener";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bonsai {
    pub id: String,
    pub birth_date: u64, // block height at which the bonsai was created
    pub thirsty: bool, // if it need to drink some water
    pub price: Coin,
}

impl Bonsai {
    // not a method but an associate function
    pub fn new(birth_date: u64, thirsty: bool, price: Coin) -> Bonsai {
        let id : String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .collect();

        Bonsai {
            id,
            birth_date,
            thirsty,
            price
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct BonsaiList {
    pub bonsais: Vec<Bonsai>
}

impl BonsaiList {
    /// grow some bonsais from a given number, watering each one of those
    pub fn grow_bonsais(water: bool, number: u32, birth_date: u64) -> BonsaiList {
        let mut i = 0;
        let mut bonsai_list = BonsaiList{ bonsais: vec![] };
        while i < number {
            bonsai_list.bonsais.push(Bonsai::new(birth_date, water))
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
    pub bonsais: Option<Vec<Bonsai>>
}

impl Gardener {
    // associate function: constructor
    fn new(name: String, address: CanonicalAddr, bonsais: Option<Vec<Bonsai>>) -> Gardener {
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
pub fn gardeners_store_readonly<S: Storage>(storage: &mut S) -> ReadonlyBucket<S, Gardener> {
    bucket_read(GARDENERS_KEY, storage)
}

#[cfg(test)]
mod tests {
    ///TODO test here
}
