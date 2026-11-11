import { readdirSync, statSync } from "node:fs";
const d = "Verträge/proposals";
try {
  console.log(readdirSync(d).map((f) => f + " " + statSync(d + "/" + f).size).join("\n"));
} catch (e) { console.log("no dir"); }
