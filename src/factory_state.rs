use std::{
    fmt,
    fs::{self, File},
    path::PathBuf,
};

use anyhow::Result;
use bdk::{
    bitcoin::{self, secp256k1::Secp256k1, Address},
    database::MemoryDatabase,
    descriptor::ExtendedDescriptor,
    wallet::AddressIndex,
    Wallet,
};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{gpg_clearsign, util, util::Desc, Database, Entry};

#[derive(Serialize, Deserialize, Debug)]
pub struct Factory {
    pub descriptor: Desc,
    pub next_index: u32,
    pub number_to_generate: u32,
    pub next_address: Address,
    pub message: String,
    pub network: bitcoin::Network,
    pub config_dir: PathBuf,
}

impl fmt::Display for Factory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Descriptor: {}", self.descriptor)?;
        writeln!(f, "Next index: {}", self.next_index)?;
        writeln!(f, "Number to generate: {}", self.number_to_generate)?;
        writeln!(f, "Next address: {}", self.next_address)?;
        writeln!(f, "Message: {}", self.message)?;
        write!(f, "Network: {}", self.network)
    }
}

impl Factory {
    /// Create a struct containing all the information necessary to derive more addresses
    pub fn new(
        descriptor: String,
        network: bitcoin::Network,
        next_index: u32,
        number_to_generate: u32,
        message: String,
        config_dir: PathBuf,
    ) -> Result<Self> {
        let secp = Secp256k1::new();
        let (desc, _) = ExtendedDescriptor::parse_descriptor(&secp, &descriptor)?;

        let next_address = util::nth_address(desc.clone(), network, next_index)?;

        Ok(Self {
            descriptor: desc,
            next_index,
            number_to_generate,
            next_address,
            message,
            network,
            config_dir,
        })
    }

    /// Load address-factory.json
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let state_json = fs::read_to_string(path)?;
        let gen_state = serde_json::from_str(&state_json)?;

        Ok(gen_state)
    }

    /// Increment the next_index
    pub fn finish(&mut self, peek_next_address: Address) {
        let old_next_index = self.next_index;
        self.next_index = old_next_index + self.number_to_generate;
        self.next_address = peek_next_address;
    }

    /// Save the struct as .json
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(self.config_dir.clone())?;
        let config_file_path = self.config_dir.join("address-factory.json");
        let f = File::create(config_file_path)?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    /// Check that first address derived matches given address
    pub fn check_next_address(&self) -> Result<Address> {
        util::check_address(
            self.descriptor.clone(),
            self.network,
            self.next_address.clone(),
            self.next_index,
        )
    }

    pub fn generate_addresses(&mut self) -> Result<()> {
        // This only peeks at the next address
        self.check_next_address()?;
        let desc = self.descriptor.clone();

        let wallet = Wallet::new_offline(desc, None, self.network, MemoryDatabase::default())?;

        if self.next_index > 0 {
            println!("Skipping addresses...");
            // This mutates the wallet to be the index right before we want to begin
            wallet.get_address(AddressIndex::Reset(self.next_index - 1))?;
        }

        // Create a new SQLite db file and connect to it
        let db = Database::new()?;

        // Don't want people staring at a blank prompt for minutes
        let pb = ProgressBar::new(self.number_to_generate as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed}] [{bar:40.green}] {pos}/{len} (eta: {eta})")
                .progress_chars("#>-"),
        );

        let mut addresses: Vec<Address> = vec![];

        println!("Generating addresses...");

        for _i in 0..self.number_to_generate {
            // Each time we ask for a New address BDK advances the index by 1
            let address = wallet.get_address(AddressIndex::New)?;
            addresses.push(address);
            pb.inc(1);
        }

        pb.finish();

        let message_text = &self.message;

        // Would be nice to do this in parallel with rayon but gpg doesn't like that
        println!("PGP signing addresses...");
        let pb = ProgressBar::new(self.number_to_generate as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed}] [{bar:40.green}] {pos}/{len} (eta: {eta})")
                .progress_chars("#>-"),
        );
        for address in addresses {
            let address = address.to_string();
            let signed_message = gpg_clearsign(&address.to_string(), message_text).unwrap();
            pb.inc(1);
            let entry = Entry::new(&address, &signed_message);
            db.insert(entry)?;
        }

        pb.finish();

        self.finish(wallet.get_address(AddressIndex::New)?);
        self.save()?;

        println!(
            "Wrote {} addresses and PGP signed messages to {}",
            self.number_to_generate, db.filename
        );

        println!(
            "Saved this setup to {}",
            self.config_dir
                .join("address-factory.json")
                .to_string_lossy()
        );
        println!("We'll use that file next time to pick up where you left off.",);

        Ok(())
    }
}
