"use strict";

import { createSign, generateKeyPairSync } from "node:crypto";

// Mirrors issueLicenseToken() in worker/index.js: the signature covers the base64url body.
export function testKeyPair() {
  const { publicKey, privateKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
  return {
    publicPem: publicKey.export({ type: "spki", format: "pem" }),
    sign(claims) {
      const body = Buffer.from(JSON.stringify(claims), "utf8").toString("base64url");
      const signer = createSign("RSA-SHA256");
      signer.update(body);
      signer.end();
      const signature = signer.sign(privateKey);
      return body + "." + signature.toString("base64url");
    },
  };
}

export function claims(overrides = {}) {
  return {
    tier: "pro",
    seatId: "seat_1",
    machineId: "web-0123456789abcdef",
    email: null,
    exp: Math.floor(Date.now() / 1000) + 3600,
    ...overrides,
  };
}