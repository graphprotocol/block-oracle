import { BigInt } from "@graphprotocol/graph-ts";

export const PREAMBLE_BIT_LENGTH = 8;
export const PREAMBLE_BYTE_LENGTH = PREAMBLE_BIT_LENGTH / 8;
export const TAG_BIT_LENGTH = 4;
//export const OWNER_ADDRESS_STRING = "{{owner}}";
export const EPOCH_MANAGER_ADDRESS = "{{epochManager}}";

export let INITIAL_PERMISSION_SET = new Map<String,Array<Array<String>>>();
{{#permissionList}}
INITIAL_PERMISSION_SET.set("{{address}}", [["{{validThrough}}"],[{{#permissions}}"{{entry}}"{{^lastEntry}},{{/lastEntry}}{{/permissions}}]])
{{/permissionList}}
export let BIGINT_ZERO = BigInt.fromI32(0);
export let BIGINT_ONE = BigInt.fromI32(1);
export let PRELOADED_ALIASES = new Map<String, String>();
PRELOADED_ALIASES.set("bip122:000000000019d6689c085ae165831e93", "btc")
PRELOADED_ALIASES.set("eip155:1", "mainnet")
PRELOADED_ALIASES.set("eip155:5", "goerli")
PRELOADED_ALIASES.set("eip155:10", "optimism")
PRELOADED_ALIASES.set("eip155:56", "bsc")
PRELOADED_ALIASES.set("eip155:97", "chapel")
PRELOADED_ALIASES.set("eip155:99", "poa-core")
PRELOADED_ALIASES.set("eip155:100", "gnosis")
PRELOADED_ALIASES.set("eip155:122", "fuse")
PRELOADED_ALIASES.set("eip155:137", "matic")
PRELOADED_ALIASES.set("eip155:250", "fantom")
PRELOADED_ALIASES.set("eip155:280", "zksync-era-testnet")
PRELOADED_ALIASES.set("eip155:288", "boba")
PRELOADED_ALIASES.set("eip155:1023", "clover")
PRELOADED_ALIASES.set("eip155:1284", "moonbeam")
PRELOADED_ALIASES.set("eip155:1285", "moonriver")
PRELOADED_ALIASES.set("eip155:1287", "mbase")
PRELOADED_ALIASES.set("eip155:4002", "fantom-testnet")
PRELOADED_ALIASES.set("eip155:42161", "arbitrum-one")
PRELOADED_ALIASES.set("eip155:42220", "celo")
PRELOADED_ALIASES.set("eip155:43113", "fuji")
PRELOADED_ALIASES.set("eip155:43114", "avalanche")
PRELOADED_ALIASES.set("eip155:44787", "celo-alfajores")
PRELOADED_ALIASES.set("eip155:80001", "mumbai")
PRELOADED_ALIASES.set("eip155:17000", "holesky")
PRELOADED_ALIASES.set("eip155:1313161554", "aurora")
PRELOADED_ALIASES.set("eip155:1313161555", "aurora-testnet")
PRELOADED_ALIASES.set("eip155:1666600000", "harmony")
PRELOADED_ALIASES.set("eip155:84532", "base-sepolia")
PRELOADED_ALIASES.set("eip155:300", "zksync-era-sepolia")
PRELOADED_ALIASES.set("eip155:1101", "polygon-zkevm")
PRELOADED_ALIASES.set("eip155:324", "zksync-era")
PRELOADED_ALIASES.set("eip155:11155111", "sepolia")
PRELOADED_ALIASES.set("eip155:421613", "arbitrum-goerli")
PRELOADED_ALIASES.set("eip155:421614", "arbitrum-sepolia")
PRELOADED_ALIASES.set("eip155:1442", "polygon-zkevm-testnet")
PRELOADED_ALIASES.set("eip155:59144", "linea")
PRELOADED_ALIASES.set("eip155:59140", "linea-goerli")
PRELOADED_ALIASES.set("eip155:8453", "base")
PRELOADED_ALIASES.set("eip155:534351", "scroll-sepolia")
PRELOADED_ALIASES.set("eip155:534352", "scroll")
PRELOADED_ALIASES.set("eip155:12611", "astar-zkevm-sepolia")
PRELOADED_ALIASES.set("eip155:81457", "blast-mainnet")
PRELOADED_ALIASES.set("eip155:3776", "astar-zkevm-mainnet")
PRELOADED_ALIASES.set("eip155:713715", "sei-testnet")
PRELOADED_ALIASES.set("eip155:168587773", "blast-testnet")