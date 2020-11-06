pub mod contract;
pub mod msg;
pub mod state;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod state_tests;

// This includes custom errors
mod errors;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);
