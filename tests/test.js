const ledger = require('./ledger-node');
const Zemu = require('@zondax/zemu');
const path = require('path');
const assert = require('assert');

const catchExit = async () => {
  process.on("SIGINT", () => {
    Zemu.stopAllEmuContainers(function () {
      process.exit();
    });
  });
};

describe("LEDGER TEST", function () {
  this.timeout(50000);

  var sim,
      transport;

  before(async function() {
    // runs before tests start
    await catchExit();
    await Zemu.default.checkAndPullImage();
    await Zemu.default.stopAllEmuContainers();

    sim = new Zemu.default(path.join(__dirname,'/node_modules/@zondax/zemu/bin/demoAppS.elf'));
    const APP_SEED = "equip will roof matter pink blind book anxiety banner elbow sun young";
    const sim_options = {
        ...Zemu.DEFAULT_START_OPTIONS,
        logging: true,
        custom: `-s "${APP_SEED}"`,
        press_delay: 150,
        model: 'nanos',
        //,X11: true
    };

    await sim.start(sim_options);

    transport = sim.getTransport();
  });

  after(async function() {
    // runs after all the test are done
    await sim.close();
    // reset
    transport = null;
  })

  it("#deviceInfo()", async function() {
    const resp = await ledger.deviceInfo(transport);

    console.log(resp);

    assert("targetId" in resp);
    assert("seVersion" in resp);
    assert("flag" in resp);
    assert("mcuVersion" in resp);
  });
})
