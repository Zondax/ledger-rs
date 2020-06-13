const ledger = require('./ledger-node')
const Zemu = require('@zondax/zemu').default
const path = require('path')

//Zemu.checkAndPullImage();

process.on("SIGINT", () => {
  Zemu.stopAllEmuContainers(function () {
    process.exit();
  });
});

const sim = new Zemu(path.join(__dirname,'/node_modules/@zondax/zemu/bin/demoApp/app.elf'));
const APP_SEED = "equip will roof matter pink blind book anxiety banner elbow sun young";

const sim_options = {
    logging: true,
    custom: `-s "${APP_SEED}"`,
    press_delay: 150
    , X11: true
};

sim.start(sim_options)
  .then(async () => {
    var transport = sim.getTransport();
    await sim.close();
  })
