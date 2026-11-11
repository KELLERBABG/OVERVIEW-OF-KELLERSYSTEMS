import { loadDatabase, verifyMappings } from "../../src/clausedb.js";
import { RULES } from "../../src/analyze.js";
const ROOT = new URL("../../", import.meta.url).pathname;
const db = loadDatabase(decodeURIComponent(ROOT.replace(/^\/([A-Za-z]:)/, "$1")));
const v = verifyMappings(db.records || db);
console.log("gapHits " + v.gapHits.length + " blind " + v.blind + " mappedVerifiedRate " + v.mappedVerifiedRate);
const supp = {
  penalty: /(?:contractual\s+penalt(?:y|ies)|liquidated\s+damages|penalt(?:y|ies)\s+of\s+[\d.,]+\s*(?:eur|usd|euro|dollars?))/i,
  jurisdiction: /(?:courts?|venue|submits?)[^.!?;:\n]{0,90}?exclusive\s+jurisdiction|exclusive\s+jurisdiction\s+(?:of|in)\s+(?:the\s+courts?|any\s+court)/i,
  non_compete: /shall\s+not[^.!?;:\n]{0,60}?(?:engage\s+in|carry\s+on|be\s+involved\s+in|participate\s+in)[^.!?;:\n]{0,60}?compet(?:ing|itive)|competing\s+business|\bnon[\s-]?compete\b/i,
  excessive_survival: /survive[^.!?;:\n]{0,80}?(?:indefinitely|in\s+perpetuity|without\s+(?:any\s+)?(?:time\s+)?limitation)/i,
  liability: /wird\s+nicht\s+gehaftet|haftung[^.!?;:\n]{0,60}?ist\s+nicht\s+er(?:ö|oe)ffnet|(?:liability|damages)[^.!?;:\n]{0,60}?(?:is\s+|shall\s+be\s+)?(?:excluded|disclaimed|not\s+assumed)/i,
  setoff_restriction: /gehindert[^.!?;:\n]{0,60}?(?:aufzurechnen|zur\s+aufrechnung|zur(?:ü|ue)ckzubehalten)|(?:nicht\s+gestattet|untersagt|verwehrt|nicht\s+befugt)[^.!?;:\n]{0,60}?(?:aufzurechnen|zur(?:ü|ue)ckzubehalten)/i,
};
const byId = new Map((db.records || db).map((r) => [r.id, r]));
let caused = 0;
for (const item of v.gapHits) {
  const rec = byId.get(item.id);
  const text = rec ? rec.clauseText : "";
  const mine = Object.entries(supp).filter(([, re]) => re.test(text)).map(([t]) => t);
  const flag = mine.length ? "MINE[" + mine.join(",") + "]" : "";
  if (mine.length) caused++;
  console.log(item.id + " " + item.clauseType + " -> " + item.found.join(",") + " " + flag);
}
console.log("gapHits caused by supplement patterns: " + caused + " of " + v.gapHits.length);
