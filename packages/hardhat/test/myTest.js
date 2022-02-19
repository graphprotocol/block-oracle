const { ethers } = require('hardhat')
const { use, expect } = require('chai')
const { solidity } = require('ethereum-waffle')

use(solidity)

describe('EpochOracle', function() {
  let epochOracle

  before(done => {
    setTimeout(done, 2000)
  })

  describe('YourContract', function() {
    it('Should deploy YourContract', async function() {
      const EpochOracle = await ethers.getContractFactory('EpochOracle')
      epochOracle = await EpochOracle.deploy()
    })

    describe('setPurpose()', function() {
      it('Should be able to set a new purpose', async function() {})
    })
  })
})
