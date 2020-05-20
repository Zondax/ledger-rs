const ledger = require('./ledger-node')
const Zemu = require('@zondax/zemu').default

// Error: elfPath cannot be null!
// Need a default app to use here...
const sim = new Zemu();
const APP_SEED = "equip will roof matter pink blind book anxiety banner elbow sun young";

const sim_options = {
    logging: true,
    custom: `-s "${APP_SEED}"`,
    press_delay: 150
    //,X11: true
};

sim.start(sim_options)
  .then(async () => {
    var transport = sim.getTransport();
    await sim.close();
  })
