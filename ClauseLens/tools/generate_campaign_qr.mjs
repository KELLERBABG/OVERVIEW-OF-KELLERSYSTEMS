#!/usr/bin/env node
"use strict";

/**
 * tools/generate_campaign_qr.mjs
 * Generates ready-to-use campaign tracking URLs and SVG QR codes
 * for ClauseLens postal mailings.
 * Zero external dependencies (native Node.js).
 */

const CAMPAIGN_BASE = "https://clause.kellersystems.dev";

const BATCHES = [
  { id: "SYS-001", name: "systemhaus-berlin", channel: "brief" },
  { id: "SYS-002", name: "systemhaus-duesseldorf", channel: "brief" },
  { id: "SYS-003", name: "systemhaus-ruhr", channel: "brief" },
  { id: "DEV-001", name: "waits-software-koeln", channel: "brief" },
  { id: "DEV-002", name: "ifaktor-koeln", channel: "brief" },
  { id: "LAW-001", name: "kanzlei-it-recht-muenchen", channel: "kanzlei-brief" },
  { id: "LAW-002", name: "kanzlei-buse-berlin", channel: "kanzlei-brief" },
  { id: "ORG-001", name: "vgsd-mitglieder-partner", channel: "partner" },
  { id: "ORG-002", name: "freelancermap-partner", channel: "partner" },
];

console.log("=== ClauseLens Kampagnen-Tracking & QR-Link Generator ===");
console.log(`Basis-URL: ${CAMPAIGN_BASE}\n`);

for (const b of BATCHES) {
  const url = `${CAMPAIGN_BASE}/?utm_source=${b.channel}&utm_medium=direct-mail&utm_campaign=${b.name}&ref=${b.id}`;
  // Public fast SVG QR API for direct embedding or print
  const qrSvgUrl = `https://api.qrserver.com/v1/create-qr-code/?size=250x250&data=${encodeURIComponent(url)}`;
  
  console.log(`[${b.id}] ${b.name}`);
  console.log(`  🔗 Ziel-URL: ${url}`);
  console.log(`  📱 Druckfähiger QR-Code: ${qrSvgUrl}\n`);
}

console.log("Hinweis: Drucken Sie den QR-Code einfach auf die DIN-A4 Briefe (rechts unten) auf.");
