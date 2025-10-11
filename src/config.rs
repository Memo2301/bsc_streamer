use ethers::types::Address;
use std::str::FromStr;

// PancakeSwap V2 Factory
pub const PANCAKESWAP_V2_FACTORY: &str = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73";

// Four.meme bonding curve contract
pub const FOURMEME_BONDING_CURVE: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";

// Base tokens on BSC
pub struct BaseToken {
    pub symbol: &'static str,
    pub address: &'static str,
}

pub const BASE_TOKENS: &[BaseToken] = &[
    BaseToken {
        symbol: "WBNB",
        address: "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c",
    },
    BaseToken {
        symbol: "BUSD",
        address: "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56",
    },
    BaseToken {
        symbol: "USDT",
        address: "0x55d398326f99059fF775485246999027B3197955",
    },
    BaseToken {
        symbol: "USDC",
        address: "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
    },
    BaseToken {
        symbol: "ETH",
        address: "0x2170Ed0880ac9A755fd29B2688956BD959F933F8",
    },
    BaseToken {
        symbol: "BTCB",
        address: "0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c",
    },
    BaseToken {
        symbol: "FOURMEME",
        address: "0x9eb5d5731dff7c3c53cf6ba3c05fc1247c790ef9",
    },
];

pub fn get_factory_address() -> Address {
    Address::from_str(PANCAKESWAP_V2_FACTORY).unwrap()
}

pub fn get_bonding_curve_address() -> Address {
    Address::from_str(FOURMEME_BONDING_CURVE).unwrap()
}

pub fn get_base_tokens() -> Vec<(String, Address)> {
    BASE_TOKENS
        .iter()
        .map(|t| (t.symbol.to_string(), Address::from_str(t.address).unwrap()))
        .collect()
}

