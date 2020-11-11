pub mod contract;
pub mod msg;
pub mod state;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod state_tests;

// This includes custom errors
pub mod errors;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
