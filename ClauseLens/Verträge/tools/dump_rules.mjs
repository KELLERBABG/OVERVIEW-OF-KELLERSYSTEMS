import { RULES } from "../../src/analyze.js";
const want = ["agency_commission_exclusion","international_data_processing","data_lockin","third_party_indemnity","asymmetric_termination","vicarious_agents","force_majeure_asymmetry","price_escalation","maintenance_conditional_shutdown","early_termination_charge","service_credit_forfeiture","extended_retention_of_title","liability","setoff_restriction","penalty","jurisdiction","non_compete","excessive_survival","agency_goodwill_waiver"];
for (const type of want) {
  const r = RULES.find((x) => x.type === type);
  console.log("### " + type + (r ? "" : " NOT FOUND"));
  if (!r) continue;
  console.log("  severity: " + (typeof r.severity === "function" ? "fn" : r.severity) + (r.collect ? "  collect: yes" : ""));
  (r.patterns || []).forEach((re, i) => console.log("  [" + i + "] " + re.source + "  flags=" + re.flags));
}
