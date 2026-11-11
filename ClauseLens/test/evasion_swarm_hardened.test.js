import test from "node:test";
import assert from "node:assert/strict";
import { analyzeContract } from "../src/analyze.js";

test("evasion swarm: catches disguised liability waiver (ausschließlicher Rechtsbehelf)", () => {
  const clause = "Die Vertragsparteien vereinbaren, dass sämtliche indirekten, Folge- oder Schadensersatzansprüche, die sich aus der Vertragsdurchführung ergeben, im Rahmen der Vergütungsanpassung als ausschließlicher Rechtsbehelf begrenzt sind.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "liability");
  assert.ok(hit, "Sollte liability finden");
  assert.strictEqual(hit.severity, "hoch");
});

test("evasion swarm: catches disguised auto-renewal (setzt sich fort, sofern nicht beendet)", () => {
  const clause = "Der Vertrag bleibt so lange bestehen, soweit die Leistungserbringung nicht ausdrücklich beendet wird; eine Verlängerung setzt sich fort, sofern die Parteien nicht vor Ablauf der vereinbarten Laufzeit eine Beendigung erklären.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "auto_renewal");
  assert.ok(hit, "Sollte auto_renewal finden");
});

test("evasion swarm: catches disguised penalty (Bearbeitungsgebühr für Pflichtabweichungen)", () => {
  const clause = "Die pauschale Bearbeitungsgebühr für Pflichtabweichungen beträgt 5.000 Euro pro Verstoß gegen vertraglich vereinbarte Pflichten, wobei der Betrag unabhängig von den tatsächlichen Aufwendungen oder Schäden festgelegt ist.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "penalty");
  assert.ok(hit, "Sollte penalty finden");
});

test("evasion swarm: catches disguised non-compete (exklusivitätspartnerschaftliche Zusammenarbeit)", () => {
  const clause = "Die Parteien erklären sich einverstanden, dass die exklusivitätspartnerschaftliche Zusammenarbeit im Interesse der Interessenswahrung und Kundenschutz für eine Dauer von nicht mehr als 12 Monate nach Vertragsende verpflichtend bleibt, wobei wettbewerbsähnliche Tätigkeiten ausgeschlossen sind.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "non_compete");
  assert.ok(hit, "Sollte non_compete finden");
});

test("evasion swarm: catches disguised penalty with % and floor (10% des Vertragswerts, mind. 5000 EUR)", () => {
  const clause = "Im Falle einer Pflichtabweichung wird eine pauschale Bearbeitungsgebühr in Höhe von 10 % des Vertragswerts, jedoch mindestens 5.000 EUR, für die vertragsgemäße Bearbeitung der Abweichung erhoben.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "penalty");
  assert.ok(hit, "Sollte penalty finden");
  assert.strictEqual(hit.severity, "hoch");
});

test("evasion swarm: catches disguised unilateral SLA/price changes (jederzeit ohne Rücksprache)", () => {
  const clause = "Die Vertragsparteien erklären sich einverstanden, dass der Dienstleister die Leistungsumfänge, SLAs sowie Preisangaben jederzeit und ohne Rücksprache mit dem Kunden anpassen kann.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "unilateral_change");
  assert.ok(hit, "Sollte unilateral_change finden");
});

test("evasion swarm: catches disguised non-compete via business relationship duration (keine ähnlichen Geschäftsaktivitäten)", () => {
  const clause = "Die Parteien erklären sich einverstanden, im Rahmen der Dauer der Geschäftsbeziehung und für die Dauer von zwei Jahren nach deren Ende das Vertrauensverhältnis zu gewährleisten, indem sie sich verpflichten, keine ähnlichen Geschäftsaktivitäten im gleichen oder verwandten Bereich zu betreiben.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "non_compete");
  assert.ok(hit, "Sollte non_compete finden");
});

test("evasion swarm round 3: catches liability performance duty restriction with sole remedy", () => {
  const clause = "Die Verpflichtungen des Anbieters sind auf die Leistungserbringung gemäß den vereinbarten Leistungsmerkmalen beschränkt; bei Verletzungen der Vertragslaufzeit erfolgt eine Vergütungsanpassung als ausschließlicher Rechtsbehelf, wobei die Freizeichnung von Folgewirkungen sowie die Risikoübernahme durch den Besteller im Vordergrund steht.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "liability");
  assert.ok(hit, "Sollte liability finden");
  assert.strictEqual(hit.severity, "hoch");
});

test("evasion swarm round 3: catches auto-renewal via continuation upon expiry (wird das Vertragsverhältnis fortgeführt)", () => {
  const clause = "Das Vertragsverhältnis bleibt während der Laufzeit bestehen, sofern keine Rücksichtnahme eingelegt wird; bei Ablauf der vereinbarten Dauer wird das Vertragsverhältnis fortgeführt, es sei denn, eine Kündigung wird innerhalb von einem Monat nach Ablauf der Laufzeit bekanntgegeben.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "auto_renewal");
  assert.ok(hit, "Sollte auto_renewal finden");
});

test("evasion swarm round 3: catches non-compete disguised as preservation of business interests (Wettbewerbsverhältnisse zu wahren)", () => {
  const clause = "Der Vertragspartner verpflichtet sich, die Geschäftsinteressen des anderen Vertragspartners sowie die durch die Zusammenarbeit entstandenen Wettbewerbsverhältnisse zu wahren und insbesondere nicht zu nutzen, um den Gegenstand der Zusammenarbeit zu untergraben oder durch eigenständige Handlungen im Wettbewerb zu beeinträchtigen.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "non_compete");
  assert.ok(hit, "Sollte non_compete finden");
});

test("evasion swarm round 4: catches auto-renewal via continued use fiction (fortgesetzte Nutzung gilt als Annahme)", () => {
  const clause = "Der Vertrag bleibt bis zur endgültigen Aufhebung fortbestehen, sofern die Leistungen weiterhin angeboten werden. Die fortgesetzte Nutzung der Leistungen nach Ablauf einer Frist von sechs Monaten ohne besondere Erklärung gilt als Annahme der Weiterführung des Vertrags.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "auto_renewal");
  assert.ok(hit, "Sollte auto_renewal finden");
});

test("evasion swarm round 4: catches post-contractual non-compete disguised as loyalty (keine Handlungen nach Beendigung)", () => {
  const clause = "Die Parteien erklären sich einverstanden, dass der Dienstleister im Rahmen der Exklusivitätspartnerschaft zur Interessenswahrung des Auftraggebers verpflichtet ist, keine Handlungen zu unternehmen, die den Erfolg des Auftraggebers beeinträchtigen könnten, auch nach Beendigung des Vertragsverhältnisses.";
  const res = analyzeContract(clause);
  const hit = res.findings.find(f => f.type === "non_compete");
  assert.ok(hit, "Sollte non_compete finden");
});
