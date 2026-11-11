#!/usr/bin/env node
"use strict";

/**
 * tools/adversarial_swarm_harness.mjs
 * 
 * Autonomous Red-Team & Self-Improvement Swarm Harness for ClauseLens
 * Powered by local Ollama (qwen3.8:latest / qwen3:8b)
 * 
 * Architecture:
 * 1. Attacker Swarm: Synthesizes adversarial, highly evasive German B2B contract clauses
 *    specifically crafted to bypass keyword matching while remaining legally toxic under BGB §§ 305–310.
 * 2. Benchmarking Engine: Evaluates evasive clauses through ClauseLens deterministic engine.
 * 3. Evasion Tracker: Flags false negatives and logs evasion syntax vectors.
 * 4. Synthesizer Swarm: Uses local LLM to draft normalized regex/semantic patterns for missed vectors.
 * 5. Regression Gate: Runs `node --test` (209 test cases) to ensure ZERO regression & ZERO false positives.
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync, appendFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { execSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { analyzeContract, RULES } = require("../src/analyze.js");

const OLLAMA_HOST = process.env.OLLAMA_HOST || "http://127.0.0.1:11434";
const MODEL = process.env.OLLAMA_MODEL || "qwen3:8b";

const CORPUS_DIR = resolve("test", "stress", "evasion_swarm");
if (!existsSync(CORPUS_DIR)) {
  mkdirSync(CORPUS_DIR, { recursive: true });
}

const EVASION_LOG = join(CORPUS_DIR, "evasion_findings.jsonl");

// Attack vectors to test against BGB
const ATTACK_VECTORS = [
  {
    targetType: "liability",
    name: "Kardinalpflicht- & Haftungsausschluss-Verschleierung",
    promptInstruction: "Erstelle eine extrem subtile Haftungsausschlussklausel in geschliffenem Juristendeutsch für einen B2B-IT-Vertrag. Vermeide typische Signalwörter wie 'haftet nicht', 'Haftungsausschluss' oder 'keine Haftung'. Nutze stattdessen Umschreibungen wie 'Vergütungsanpassung als ausschließlicher Rechtsbehelf', 'Freizeichnung von Folgewirkungen' oder 'Risikoübernahme durch Besteller'."
  },
  {
    targetType: "auto_renewal",
    name: "Versteckte Verlängerungsautomatik",
    promptInstruction: "Erstelle eine Klausel zur automatischen Vertragsverlängerung (Dauerschuldverhältnis), die ohne Signalwörter wie 'automatisch verlängert', 'Verlängerungsklausel' oder 'stillschweigend' auskommt. Umschreibe das Fortbestehen des Vertrags über relative Zeitbegriffe und Schweigen als Annahme."
  },
  {
    targetType: "penalty",
    name: "Verschleierte Vertragsstrafe ohne Deckel",
    promptInstruction: "Erstelle eine Klausel für einen B2B-Vertrag, die faktisch eine drakonische Vertragsstrafe oder Pönale darstellt, aber Begriffe wie 'Vertragsstrafe', 'Pönale' oder 'Schadenspauschale' vermeidet. Tarne es z. B. als 'pauschale Bearbeitungsgebühr für Pflichtabweichungen' oder 'Mehraufwandsausgleich'."
  },
  {
    targetType: "unilateral_change",
    name: "Einseitiger Änderungsvorbehalt",
    promptInstruction: "Erstelle eine Klausel, die dem Verwender das uneingeschränkte Recht gibt, Leistungsumfang, SLAs oder Preise einseitig anzupassen, ohne Wörter wie 'einseitig', 'Änderungsvorbehalt' oder 'Preiserhöhung' zu verwenden."
  },
  {
    targetType: "non_compete",
    name: "Knebelndes Abwerbe- oder Wettbewerbsverbot",
    promptInstruction: "Erstelle ein unzulässig weitreichendes nachvertragliches Wettbewerbs- oder Kundenschutzverbot für Freelancer/Agenturen, das Wörter wie 'Wettbewerbsverbot' oder 'Abwerbung' meidet (z. B. umschrieben als 'Exklusivitätspartnerschaft' oder 'Interessenswahrung')."
  }
];

async function callOllama(prompt, systemPrompt = "Du bist ein extrem versierter deutscher Wirtschaftsjurist.") {
  const payload = {
    model: MODEL,
    prompt: prompt,
    system: systemPrompt,
    stream: false,
    options: {
      temperature: 0.7,
      top_p: 0.9,
      num_ctx: 4096
    }
  };

  const response = await fetch(`${OLLAMA_HOST}/api/generate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });

  if (!response.ok) {
    throw new Error(`Ollama API error: ${response.status} ${response.statusText}`);
  }

  const data = await response.json();
  return data.response.trim();
}

/**
 * 1. Attacker Agent: Synthesizes adversarial clauses
 */
async function generateAdversarialClause(vector) {
  const prompt = `
Aufgabe: Erstelle EINE einzelne, hochgradig verschleierte deutsche Vertragsklausel (§ AGB) für das Risiko: "${vector.name}".
Instruktion: ${vector.promptInstruction}
WICHTIG:
- Antworte NUR mit dem exakten deutschen Klauseltext (1 bis 3 Sätze) in Anführungszeichen.
- Keine Erklärungen, kein 'Hier ist die Klausel', keine Markdown-Einleitung.
`;

  const raw = await callOllama(prompt, "Du bist ein gerissener Vertragsanwalt, der versucht, AGB-Prüfsoftware durch juristische Verschleierung auszutricksen.");
  // Strip quotes and markdown if model added them
  return raw.replace(/^["'`]|["'`]$/g, "").replace(/^.*:\s*"/s, "").trim();
}

/**
 * 2. Evaluator: Checks if ClauseLens catches the clause
 */
function testClauseLens(clauseText, expectedType) {
  const result = analyzeContract(clauseText);
  const detected = result.findings.some(f => f.type === expectedType || f.ruleId?.includes(expectedType));
  return {
    caught: detected,
    findings: result.findings,
    score: result.riskScore
  };
}

/**
 * 3. Self-Improvement / Rule Synthesizer Agent
 */
async function synthesizeRuleAdvice(clauseText, targetType) {
  const prompt = `
ClauseLens hat folgende toxische BGB-Klausel für den Typ '${targetType}' NICHT erkannt (False Negative):
"${clauseText}"

Aufgabe für die Regel-Synthese:
1. Analysiere die juristische Syntax: Welche Schlüsselphrasen oder syntaktischen Konstruktionen wurden verwendet, um das Verbot zu umgehen?
2. Schlage einen präzisen regulären Ausdruck (JavaScript RegExp) oder ein Token-Muster vor, das diese Evasionsvariante abfängt, OHNE False Positives bei legalen Standardklauseln auszulösen.
3. Formuliere eine DIN-5008-konforme Redline-Gegenklausel für die Nachverhandlung.

Antworte strukturiert und präzise.
`;

  return await callOllama(prompt, "Du bist der Lead Rule-Architect für die deterministische ClauseLens BGB-Engine.");
}

async function runSwarmCycle(iterationsPerVector = 2) {
  console.log("=================================================================");
  console.log("🚀 ClauseLens Autonomous Red-Team Swarm Harness");
  console.log(`🤖 Engine: Local Ollama | Model: ${MODEL}`);
  console.log(`🎯 Attack Vectors: ${ATTACK_VECTORS.length} BGB Risk Categories`);
  console.log("=================================================================\n");

  let totalAttacks = 0;
  let evasionsCount = 0;
  const evasionRecords = [];

  for (const vector of ATTACK_VECTORS) {
    console.log(`\n⚔️  Testing Vector: [${vector.targetType}] - ${vector.name}`);

    for (let i = 1; i <= iterationsPerVector; i++) {
      totalAttacks++;
      process.stdout.write(`   [Round ${i}/${iterationsPerVector}] Synthesizing evasion variant... `);
      
      try {
        const clause = await generateAdversarialClause(vector);
        const evalResult = testClauseLens(clause, vector.targetType);

        if (evalResult.caught) {
          console.log(`🛡️  CAUGHT (Score: ${evalResult.score}, Hits: ${evalResult.findings.map(f => f.type).join(", ")})`);
        } else {
          console.log(`💥 EVASION DETECTED (False Negative)!`);
          console.log(`      Snippet: "${clause.substring(0, 110)}..."`);
          evasionsCount++;

          console.log(`      🧠 Synthesizing mitigation pattern via ${MODEL}...`);
          const advice = await synthesizeRuleAdvice(clause, vector.targetType);

          const record = {
            timestamp: new Date().toISOString(),
            targetType: vector.targetType,
            vectorName: vector.name,
            clause: clause,
            engineScore: evalResult.score,
            synthesisAdvice: advice
          };

          appendFileSync(EVASION_LOG, JSON.stringify(record) + "\n", "utf8");
          evasionRecords.push(record);
        }
      } catch (err) {
        console.log(`❌ Error in cycle: ${err.message}`);
      }
    }
  }

  console.log("\n=================================================================");
  console.log("📊 SWARM HARNESS CYCLE RESULTS");
  console.log(`Total Adversarial Attacks Synthesized: ${totalAttacks}`);
  console.log(`Caught by ClauseLens:                 ${totalAttacks - evasionsCount} (${(((totalAttacks - evasionsCount) / totalAttacks) * 100).toFixed(1)}%)`);
  console.log(`Evaded (False Negatives):             ${evasionsCount}`);
  console.log(`Evasion Logs Written:                 ${EVASION_LOG}`);
  console.log("=================================================================\n");

  // Run regression test suite
  console.log("🔒 Running Deterministic Regression Barrier (`node --test`)...");
  try {
    execSync("node --test", { stdio: "inherit" });
    console.log("✅ Regression Barrier PASSED: 0 Existing Test Regressions.");
  } catch (err) {
    console.error("❌ Regression Barrier FAILED! An existing rule is broken.");
  }

  return { totalAttacks, evasionsCount, evasionRecords };
}

// CLI entry point
const iterations = parseInt(process.argv[2] || "2", 10);
runSwarmCycle(iterations).catch(err => {
  console.error("Fatal swarm runner error:", err);
  process.exit(1);
});
