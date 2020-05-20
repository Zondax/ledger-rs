import TransportU2F from "@ledgerhq/hw-transport-u2f";
import * as ledger from "./ledger-browser";

function log(text) {
  document.getElementById("output").innerHTML += text + "\n";
}

log("\n...Trying to connect to ledger...\n");

TransportU2F.create(10000)
  .then(async (transport) => {
    log("\n...Got transport...\n");

    // We need this scramble key
    transport.setScrambleKey("FIL");

    log("\n...Calling Device Info...\n");

    // FIXME: need to open an app to get device info ?
    // In node, no need to open an app to get this information
    const info = await ledger.deviceInfo(transport);

    log(`${JSON.stringify(info, 0, 4)}`);

  })
