use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Coin, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static BONSAI_KEY: &[u8] = b"bonsai";
pub static GARDENERS_KEY: &[u8] = b"gardener";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bonsai {
    pub id: u64,
    // block height at which the bonsai was created
    pub birth_date: u64,
    pub price: Coin,
}

impl Bonsai {
    // not a method but an associate function
    pub fn new(id: u64, birth_date: u64, price: Coin) -> Bonsai {
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
    pub fn grow_bonsais(number: u64, birth_date: u64, price: Coin) -> BonsaiList {
        let mut i = 0;
        let mut bonsais: Vec<Bonsai> = Vec::with_capacity(number as usize);
        while i < number {
            bonsais.push(Bonsai::new(i, birth_date, price.clone()));
            i += 1;
        }
        BonsaiList{ bonsais }
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
    bucket(storage, GARDENERS_KEY)
}

/// return a read-only gardeners' bucket
pub fn gardeners_store_readonly<S: Storage>(storage: &S) -> ReadonlyBucket<S, Gardener> {
    bucket_read(storage, GARDENERS_KEY)
}
