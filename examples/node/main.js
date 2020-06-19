const ledger = require('./ledger-node')
const TransportNodeHid = require('@ledgerhq/hw-transport-node-hid').default

TransportNodeHid.create()
  .then(async (transport) => {
    console.log(`We are connected to your ${transport.deviceModel.productName}\nGetting Device information now!`)
    const info = await ledger.deviceInfo(transport)
    console.log(info)
  })
