module.exports = async ({ getNamedAccounts, deployments }) => {
  const { deploy } = deployments
  const { deployer } = await getNamedAccounts()

  await deploy('EpochOracle', {
    from: deployer,
    args: [deployer],
    log: true,
    waitConfirmations: 5,
  })
}
module.exports.tags = ['EpochOracle']
