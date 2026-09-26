import assert from "node:assert/strict";
import { test } from "node:test";
import * as StellarSdk from "@stellar/stellar-sdk";

import {
  buildRaiseDisputeTx,
  buildResolveTx,
  submitSorobanTransaction,
} from "./dispute-tx.ts";

const testAddress = "GCEZUZQNH2IPVGPI3KDB2PEXCRRPXOGBRY4ETCAHOJXZEFQ5BKBFH3YO";
const contractId = "CAXSLOYSXT7DETC26ED52V2LZ4SNCHSM6PI7JZ2Y36N2NZS4ZCPLP3GO";
const reasonHex = "a".repeat(64);
const evidenceHex = "b".repeat(64);

test("buildRaiseDisputeTx prepares a valid invoke contract transaction", async () => {
  const originalGetAccount = StellarSdk.rpc.Server.prototype.getAccount;
  const originalPrepare = StellarSdk.rpc.Server.prototype.prepareTransaction;

  try {
    StellarSdk.rpc.Server.prototype.getAccount = async () =>
      new StellarSdk.Account(testAddress, "100");
    StellarSdk.rpc.Server.prototype.prepareTransaction = async (tx) => tx;

    const xdr = await buildRaiseDisputeTx({
      contractId,
      userAddress: testAddress,
      reasonHash: reasonHex,
      disputeEvidenceHash: evidenceHex,
    });

    assert.ok(typeof xdr === "string");
    const tx = StellarSdk.TransactionBuilder.fromXDR(
      xdr,
      "Test SDF Network ; September 2015"
    );
    assert.equal(tx.operations.length, 1);
    assert.equal(tx.operations[0].type, "invokeHostFunction");
  } finally {
    StellarSdk.rpc.Server.prototype.getAccount = originalGetAccount;
    StellarSdk.rpc.Server.prototype.prepareTransaction = originalPrepare;
  }
});

test("buildResolveTx prepares resolution for Release, Refund and Split", async () => {
  const originalGetAccount = StellarSdk.rpc.Server.prototype.getAccount;
  const originalPrepare = StellarSdk.rpc.Server.prototype.prepareTransaction;

  try {
    StellarSdk.rpc.Server.prototype.getAccount = async () =>
      new StellarSdk.Account(testAddress, "100");
    StellarSdk.rpc.Server.prototype.prepareTransaction = async (tx) => tx;

    for (const outcome of ["RELEASE", "REFUND", "SPLIT"]) {
      const xdr = await buildResolveTx({
        contractId,
        userAddress: testAddress,
        outcome,
        splitBps: outcome === "SPLIT" ? 3500 : 0,
      });

      assert.ok(typeof xdr === "string");
      const tx = StellarSdk.TransactionBuilder.fromXDR(
        xdr,
        "Test SDF Network ; September 2015"
      );
      assert.equal(tx.operations.length, 1);
      assert.equal(tx.operations[0].type, "invokeHostFunction");
    }
  } finally {
    StellarSdk.rpc.Server.prototype.getAccount = originalGetAccount;
    StellarSdk.rpc.Server.prototype.prepareTransaction = originalPrepare;
  }
});

test("buildRaiseDisputeTx rejects invalid hash lengths", async () => {
  await assert.rejects(
    async () => {
      await buildRaiseDisputeTx({
        contractId,
        userAddress: testAddress,
        reasonHash: "invalid_short_hash",
        disputeEvidenceHash: evidenceHex,
      });
    },
    { message: /El hash debe ser una cadena hexadecimal válida de 32 bytes/ }
  );
});

test("submitSorobanTransaction polls until status SUCCESS", async () => {
  const originalSend = StellarSdk.rpc.Server.prototype.sendTransaction;
  const originalGetTx = StellarSdk.rpc.Server.prototype.getTransaction;

  try {
    StellarSdk.rpc.Server.prototype.sendTransaction = async () => ({
      status: "PENDING",
      hash: "abc123txhash",
    });
    StellarSdk.rpc.Server.prototype.getTransaction = async () => ({
      status: "SUCCESS",
    });

    const account = new StellarSdk.Account(testAddress, "100");
    const contract = new StellarSdk.Contract(contractId);
    const op = contract.call(
      "resolve",
      StellarSdk.xdr.ScVal.scvVec([StellarSdk.xdr.ScVal.scvSymbol("Release")]),
      StellarSdk.xdr.ScVal.scvU32(0)
    );
    const tx = new StellarSdk.TransactionBuilder(account, {
      fee: "1000",
      networkPassphrase: "Test SDF Network ; September 2015",
    })
      .addOperation(op)
      .setTimeout(30)
      .build();

    const result = await submitSorobanTransaction({
      signedXdr: tx.toXDR(),
    });

    assert.equal(result.success, true);
    assert.equal(result.txHash, "abc123txhash");
  } finally {
    StellarSdk.rpc.Server.prototype.sendTransaction = originalSend;
    StellarSdk.rpc.Server.prototype.getTransaction = originalGetTx;
  }
});
