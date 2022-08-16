// This is a patch for the original hardhat.config.ts file.
//
// It extends the network list to include one named "compose" which targets the `hardhat`
// container in a local docker compose project.

import config from "./hardhat.config";

const dockerComposeHardhat = {
  chainId: 1337,
  url: 'http://hardhat:8545',
};

config.networks.compose = dockerComposeHardhat;

export default config
