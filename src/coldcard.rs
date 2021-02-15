use std::str::FromStr;
use anyhow::Result;

use bdk::descriptor::get_checksum;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Bip84Json {
    _pub: String,
    deriv: String,
    pub first: String,
    name: String,
    xfp: String,
    pub xpub: String,
}

#[derive(Debug, Deserialize)]
pub struct ColdcardJson {
    chain: String,
    pub xfp: String,
    xpub: String,
    // TODO: use the account, yes?
    account: u64,
    // TODO: use other address types?
    pub bip84: Bip84Json,
}

fn build_descriptor(xpub: &str, fingerprint: &str) -> String {
    // m / purpose' / coin_type' / account' / change / index
    // TODO: use the account & other address types?
    let hardened_derivation_path = "m/84h/1h/0h";

    let origin_prefix = hardened_derivation_path.replace("m", &fingerprint.to_lowercase());

    let descriptor_part = format!("[{}]{}", origin_prefix, xpub);

    let is_change = false;
    let inner = format!("{}/{}/*", descriptor_part, is_change as u32);
    // TODO: use the selected address type (not hardcoded)
    let descriptor = format!("wpkh({})", inner);

    format!("{}#{}", descriptor, get_checksum(&descriptor).unwrap())
}

impl ColdcardJson {
    pub fn build_descriptor_string(&self) -> String {
        if &self.chain != "XTN" {
            panic!("We only support tpub right now")
        }

        build_descriptor(&self.bip84.xpub, &self.xfp)
    }

    pub fn get_first_addresss(&self) -> String {
        self.bip84.first.clone() 
    }
}

impl FromStr for ColdcardJson {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bdk::{bitcoin, descriptor::ToWalletDescriptor};

    use crate::coldcard::ColdcardJson;
    #[test]
    fn parse_coldcard_json() {
        let sample_coldcard_json = r#"
    {"chain": "XTN", "xpub": "tpubD6NzVbkrYhZ4YKdvixLTQPPQYoVTPSKHzEeJXrsWEG9b6j574WRW83Ae6cwUgV7Gm6wJMhStQFgeSycgiQd78qTwcYeqMQfCpKWKvXsHzfP", "xfp": "5806F998", "account": 0, "bip49": {"xpub": "tpubDCLarAtUfuDEQzFZ3ZPXbgHfUv3T2XXiiH5p5P74LKM2mKAZRJvpDcRyihKeGT3TeYbavNdkc3mW2HrfHRHhvDunnndBhXomL5frXszrTUt", "first": "2N3r1rSMgHfv4TGAGEnP4JeRsJyaX8gBtt3", "deriv": "m/49'/1'/0'", "xfp": "DF3AA414", "name": "p2wpkh-p2sh", "_pub": "upub5DUAQw43WadLbJVM6J3ZmVfndtWTDH4ARJvrNeNFrPTVrj4Ab7b3CynjR3N6kE8TdiiG4NLCgxsogFXMjUCRe5Qh8AncGGh4vwtWcB9zJUf"}, "bip44": {"xpub": "tpubDCii3RE7sirZ3sEuyX1jjqybzN43SpA2dBjHy3sBPFj9crxYFCPpSDUK4CbSvsdMdcRnySEVf4xxxTNfas4AdufYNeWDtc1GWVowJ1PwxPb", "first": "n13mxULobwxVPGyvSbe15t36wwiXh3yQ9c", "deriv": "m/44'/1'/0'", "xfp": "1C3B66FB", "name": "p2pkh"}, "bip84": {"xpub": "tpubDCHCHHJJweJK8t8egF6341Zg12CDNqXyXVU9NRp1KHjoPBRcbRrWvMpjqzhjWLUoyc12ArBqNV7j6trxUWDFcjfE7DASYTaP8vhYrUMcAed", "first": "tb1qhxs6sljsjykrgwzkzpehtqnpftc7x8qn38wgn0", "deriv": "m/84'/1'/0'", "xfp": "26A56F42", "name": "p2wpkh", "_pub": "vpub5YF39i8nw1FuAVZZZLXhRv3JKxofWD3v9dqQT5y6DNE9Xh8T1tgJXnqdZYhmz2DjNREW4KUqv4aae99DeFXz8pqjJw2Hh7HB1WyrKQXnC9K"}}"#;
        let sample_parsed: ColdcardJson =
            ColdcardJson::from_str(sample_coldcard_json).expect("Failed to parse json");

        let sample_desc = sample_parsed.get_descriptor(bitcoin::Network::Testnet);

        let model = "wpkh([5806f998/84h/1h/0h]tpubDCHCHHJJweJK8t8egF6341Zg12CDNqXyXVU9NRp1KHjoPBRcbRrWvMpjqzhjWLUoyc12ArBqNV7j6trxUWDFcjfE7DASYTaP8vhYrUMcAed/0/*)#sd0wc6nn";

        let model_desc = model
            .to_wallet_descriptor(bitcoin::Network::Testnet)
            .unwrap();

        assert_eq!(sample_desc.0, model_desc.0);
    }
}
