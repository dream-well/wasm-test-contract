# Bonsai contract

This is a contract that I made for study purposes of the cosmWASM framework.
The idea behind it is a bonsai 🌳 shop platform, nothing serious, probably it contains some bugs too (🐜).

CosmWASM allows you to create, compile and build smart contracts on a
cosmosSDK based blockchain.

To understand the framework better, please read the overview in the
[cosmwasm repo](https://github.com/CosmWasm/cosmwasm/blob/master/README.md),
and dig into the [cosmwasm docs](https://www.cosmwasm.com).

# How to try it
1. [Install the requested components and set up the environment](https://docs.cosmwasm.com/getting-started/installation.html).
2. Clone the project: from your CLI `git clone https://github.com/bragaz/wasm-test-contract.git`
3. Move inside the project folder: `cd ../wasm-test-contract`
4. Run the docker command to compile the contract: `docker run --rm -v "$(pwd)":/code \
                                                      --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
                                                      --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
                                                      cosmwasm/rust-optimizer:0.10.4`
5. Inside the contract `helper.ts` file there is an example of how to try out the contract
