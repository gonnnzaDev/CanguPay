import assert from "node:assert/strict";
import { test, mock } from "node:test";

import { signWithFreighterGuard } from "./sign-transaction-guard.ts";

const connectedAddress = "GCONNECTED";
const testnetPassphrase = "Test SDF Network ; September 2015";

function createHarness(overrides = {}) {
  const queryNetwork = mock.fn(async () => ({
    network: "TESTNET",
    passphrase: testnetPassphrase,
  }));
  const signTransaction = mock.fn(async () => ({
    signedTxXdr: "signed-xdr",
    signerAddress: connectedAddress,
  }));
  const onNetworkChecked = mock.fn(() => {});

  return {
    queryNetwork,
    signTransaction,
    onNetworkChecked,
    options: {
      xdr: "unsigned-xdr",
      connectedAddress,
      isConnected: true,
      expectedPassphrase: testnetPassphrase,
      queryNetwork,
      signTransaction,
      onNetworkChecked,
      ...overrides,
    },
  };
}

test("signs using the connected address and the verified network passphrase", async () => {
  const { options, signTransaction, onNetworkChecked } = createHarness();

  assert.deepEqual(await signWithFreighterGuard(options), {
    success: true,
    signedXdr: "signed-xdr",
  });
  assert.deepEqual(signTransaction.mock.calls[0].arguments, ["unsigned-xdr", {
    networkPassphrase: testnetPassphrase,
    address: connectedAddress,
  }]);
  assert.deepEqual(onNetworkChecked.mock.calls[0].arguments, ["TESTNET", testnetPassphrase]);
});

test("rejects a mismatched signer even if Freighter returns signed XDR", async () => {
  const { options } = createHarness({
    signTransaction: async () => ({ signedTxXdr: "signed-xdr", signerAddress: "GOTHER" }),
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(result.signedXdr, undefined);
  assert.match(result.error ?? "", /signerAddress.*no coincide/);
});

test("rejects missing signer identity rather than trusting unsigned metadata", async () => {
  const { options } = createHarness({
    signTransaction: async () => ({ signedTxXdr: "signed-xdr" }),
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(result.signedXdr, undefined);
});

test("returns the Freighter error message and never returns an XDR on error", async () => {
  const { options } = createHarness({
    signTransaction: async () => ({
      signedTxXdr: "signed-xdr",
      signerAddress: connectedAddress,
      error: { message: "Signing rejected" },
    }),
  });

  assert.deepEqual(await signWithFreighterGuard(options), {
    success: false,
    error: "Signing rejected",
  });
});

test("rejects empty signed XDR", async () => {
  const { options } = createHarness({
    signTransaction: async () => ({ signedTxXdr: "  ", signerAddress: connectedAddress }),
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(result.signedXdr, undefined);
});

test("rejects missing transaction XDR before opening Freighter", async () => {
  const { options, queryNetwork, signTransaction } = createHarness({ xdr: "  " });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(result.signedXdr, undefined);
  assert.equal(queryNetwork.mock.callCount(), 0);
  assert.equal(signTransaction.mock.callCount(), 0);
});

test("blocks signing if the network changed after connection", async () => {
  const { options, signTransaction, onNetworkChecked } = createHarness({
    queryNetwork: async () => ({ network: "PUBLIC", passphrase: "Public Global Stellar Network ; September 2015" }),
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(result.signedXdr, undefined);
  assert.equal(signTransaction.mock.callCount(), 0);
  assert.deepEqual(onNetworkChecked.mock.calls[0].arguments, ["PUBLIC", "Public Global Stellar Network ; September 2015"]);
});

test("blocks an inconsistent network label even if its passphrase claims Testnet", async () => {
  const { options, signTransaction } = createHarness({
    queryNetwork: async () => ({ network: "PUBLIC", passphrase: testnetPassphrase }),
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(signTransaction.mock.callCount(), 0);
});

test("blocks when the network passphrase is unavailable", async () => {
  const { options, signTransaction } = createHarness({
    queryNetwork: async () => ({ network: "TESTNET", passphrase: null }),
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(signTransaction.mock.callCount(), 0);
});

test("blocks signing when the network cannot be checked", async () => {
  const { options, signTransaction } = createHarness({
    queryNetwork: async () => { throw new Error("network unavailable"); },
  });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(result.signedXdr, undefined);
  assert.equal(signTransaction.mock.callCount(), 0);
});

test("blocks signing when the wallet is disconnected", async () => {
  const { options, signTransaction } = createHarness({ isConnected: false });

  const result = await signWithFreighterGuard(options);
  assert.equal(result.success, false);
  assert.equal(signTransaction.mock.callCount(), 0);
});

test("converts thrown Freighter errors into failed results", async () => {
  const { options } = createHarness({
    signTransaction: async () => { throw new Error("User declined"); },
  });

  assert.deepEqual(await signWithFreighterGuard(options), {
    success: false,
    error: "User declined",
  });
});
