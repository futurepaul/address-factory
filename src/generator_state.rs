use std::{fs::{self, File}, path::PathBuf};

use anyhow::{bail, anyhow, Result};
use bdk::{Wallet, bitcoin::{self, Address, secp256k1::Secp256k1}, database::MemoryDatabase, descriptor::{ExtendedDescriptor, IntoWalletDescriptor, KeyMap}};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GeneratorState {
    // TODO: could this be more strongly typed?
    descriptor: String,
    pub next_index: u64,
    pub number_to_generate: u64,
    next_address: String,
    pub message: String,
}

impl GeneratorState {
    /// Create a struct containing all the information necessary to derive more addresses
    pub fn new(descriptor: String, next_index: u64, number_to_generate: u64, next_address: String, message: String) -> Self {
        Self {
            descriptor,
            next_index,
            number_to_generate,
            next_address,
            message,
        }
    }

    /// Get path to .json from user
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let state_json = fs::read_to_string(path)?;
        let gen_state = serde_json::from_str(&state_json)?;

        Ok(gen_state)
    }

    /// Increment the next_index
    pub fn finish(&mut self, peek_next_address: String) {
        let old_next_index = self.next_index;
        self.next_index = old_next_index + self.number_to_generate; 
        self.next_address = peek_next_address;
    }

    /// Save the struct as .json
    pub fn save(&self) -> Result<()> {
        let f = File::create("signed-address-generator-state.json")?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

// Check that first address derived matches that expected (if provided)    
pub fn check_first_address(&self, wallet: &Wallet<(), MemoryDatabase>) -> Result<Address> {
        let address = wallet.get_new_address()?;
        let next = self.next_address.clone(); 
        println!("Checking first address\nDerived:  {}\nExpected: {}", address.clone(), next.clone());
        if address.to_string() == next {
            Ok(address) } else {
            bail!("Incorrect first address derived. Check descriptor / xpub / derivation path.")
        }
    }

    pub fn get_descriptor(
        &self,
        network: bitcoin::Network,
    ) -> Result<(ExtendedDescriptor, KeyMap)> {
        let secp = Secp256k1::new();
        match self.descriptor.into_wallet_descriptor(&secp, network) {
            Ok(descriptor_pair) => Ok(descriptor_pair),
            Err(e) => Err(anyhow!("{}", e)),
        }
    }
}
