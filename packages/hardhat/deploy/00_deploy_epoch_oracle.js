// deploy/00_deploy_your_contract.js

const { ethers } = require("hardhat");

module.exports = async ({ getNamedAccounts, deployments }) => {
  const { deploy } = deployments;
  const { deployer, adamLocal } = await getNamedAccounts();

  await deploy("EpochOracle", {
    // Learn more about args here: https://www.npmjs.com/package/hardhat-deploy#deploymentsdeploy
    from: deployer,
    args: [ adamLocal ],
    log: true,
    waitConfirmations: 5,
  });

};
module.exports.tags = ["EpochOracle"];
