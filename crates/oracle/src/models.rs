use serde_with::DeserializeFromStr;
use std::{fmt::Display, str::FromStr};
use web3::Web3;

#[derive(Clone, Debug)]
pub struct JrpcProviderForChain<T>
where
    T: web3::Transport,
{
    pub chain_id: Caip2ChainId,
    pub web3: Web3<T>,
}

impl<T> JrpcProviderForChain<T>
where
    T: web3::Transport,
{
    pub fn new(chain_id: Caip2ChainId, transport: T) -> Self {
        Self {
            chain_id,
            web3: Web3::new(transport),
        }
    }
}

/// See https://github.com/ChainAgnostic/CAIPs/blob/master/CAIPs/caip-2.md.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, DeserializeFromStr)]
#[repr(transparent)]
pub struct Caip2ChainId {
    chain_id: String,
}

impl Caip2ChainId {
    const SEPARATOR: char = ':';

    pub fn as_str(&self) -> &str {
        &self.chain_id
    }

    pub fn ethereum_mainnet() -> Self {
        Self::from_str("eip155:1").unwrap()
    }

    pub fn namespace_part(&self) -> &str {
        self.chain_id.split_once(Self::SEPARATOR).unwrap().0
    }

    pub fn reference_part(&self) -> &str {
        self.chain_id.split_once(Self::SEPARATOR).unwrap().1
    }
}

impl FromStr for Caip2ChainId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split(Self::SEPARATOR).collect::<Vec<&str>>();

        let is_ascii_alphanumberic_or_hyphen =
            |s: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');

        #[allow(clippy::len_zero)]
        if split.len() == 2
            && split[0].len() >= 3
            && split[0].len() <= 8
            && is_ascii_alphanumberic_or_hyphen(split[0])
            && split[1].len() >= 1
            && split[1].len() <= 32
            && is_ascii_alphanumberic_or_hyphen(split[1])
        {
            Ok(Self {
                chain_id: s.to_string(),
            })
        } else {
            Err("Invalid chain id".to_string())
        }
    }
}

impl Display for Caip2ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caip2_chain_id_valid_test_cases() {
        let valid_caip2 = vec![
            "ethereum:eip155-1",
            "bip122:000000000019d6689c085ae165831e93",
            "bip122:12a765e31ffd4059bada1e25190f6e98",
            "bip122:fdbe99b90c90bae7505796461471d89a",
            "cosmos:cosmoshub-2",
            "cosmos:cosmoshub-3",
            "cosmos:Binance-Chain-Tigris",
            "cosmos:iov-mainnet",
            "lip9:9ee11e9df416b18b",
            "chainstd:8c3444cf8970a9e41a706fab93e7a6c4",
        ];
        for s in valid_caip2 {
            assert!(Caip2ChainId::from_str(s).is_ok());
        }
    }

    #[test]
    fn caip2_chain_id_empty() {
        assert!(Caip2ChainId::from_str("").is_err());
    }

    #[test]
    fn caip2_chain_id_no_colons() {
        assert!(Caip2ChainId::from_str("foobar").is_err());
    }

    #[test]
    fn caip2_chain_id_too_long() {
        assert!(Caip2ChainId::from_str("chainstd:8c3444cf8970a9e41a706fab93e7a6c40").is_err());
        assert!(Caip2ChainId::from_str("chainstda:8c3444cf8970a9e41a706fab93e7a6c4").is_err());
    }
}
