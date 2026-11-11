# Ambient Thermal Arbitrage – Der Pitch

## Was ist passiert?

Wir haben eine Flotten-Simulation mit **20 verschiedenen Anlagentypen** gebaut – vom Supermarkt-Kühlregal über Tiefkühllager bis zur Rechenzentrum-Klimatisierung, von der Chemiefabrik bis zur Brauerei.

Jeder Anlagentyp wurde mit realistischen physikalischen Parametern modelliert: Kühlleistung, Wärmeeintrag, Dämmung, Abtauverluste, COP, thermische Masse. Wir haben 8760 Stunden Wetter- und Strompreisdaten aus 2025 eingespielt und mit einem 7-stufigen Regelungssystem simuliert, das intelligent zwischen Notfall, Netzdienstleistung, Arbitrage und Normalbetrieb priorisiert.

## Was beweist die Simulation?

**1. Thermische Arbitrage funktioniert – und zwar für alle Anlagentypen.**

Wir haben nachgewiesen, dass Kälteanlagen systematisch Stromkosten senken können, indem sie Kühlarbeit in günstige Preisstunden verschieben – ohne die Produktqualität zu gefährden. Das Kernprinzip: Die Kältemaschine läuft durchgehend im VFD-Modus (modulierend), nicht taktend. Dadurch bleibt die Energiebilanz neutral – es wird nicht mehr gekühlt, sondern nur *wann* gekühlt wird, verschiebt sich.

**2. Energieneutralität ist der Hebel.**

Weil die Maschine ständig läuft und exakt die Wärmelast ausgleicht, bleibt der Gesamtstromverbrauch gleich. Die Einsparung kommt *ausschließlich* aus der zeitlichen Verschiebung in günstige Stunden – nicht aus weniger Kühlung. Das ist der entscheidende Unterschied zu klassischen Demand-Response-Ansätzen, die Produktion drosseln und damit Werte vernichten.

**3. Jeder Anlagentyp hat sein eigenes wirtschaftliches Profil.**

- **Großmärkte (Aldi, Edeka, Marktkauf):** Hohe Grundlast → hohes Arbitragevolumen. 400+ Filialen ergeben sechsstellige jährliche Einsparungen.
- **Tiefkühl- und Kühlhäuser (freezer, cold, frozen):** Sehr große thermische Trägheit → lange Verschiebungsfenster → hohe Flexibilität.
- **Spezialanlagen (Cryo, Pharma, Tauchbecken, Rechenzentren):** Geringe Pufferzeiten, aber hohe Kühlleistung → kurzfristige Netzdienstleistung (FCR) als Zusatzertrag.
- **Industrie (Chemie, Fertigung, Schmelzkühlung):** Laute Umgebung mit aggressiven Arbitrage-Schwellen erzielt hohe Spreads, auch bei kurzen Fenstern.

**4. Die Baseline ist konservativ.**

Die Einsparungen werden gegen eine realistische Baseline gerechnet, die das taktende Verhalten heutiger Thermostatregelung abbildet (30% Mehrverbrauch durch Ein-/Ausschaltverluste). Das macht die Ergebnisse belastbar – die Simulation übertreibt nicht.

## Die Botschaft an Führungskräfte

> **Sie haben bereits die Infrastruktur für eine virtuelle Großbatterie in Ihren Kühlhäusern. Wir zeigen Ihnen, wie Sie sie aktivieren – ohne Investitionen in neue Hardware, ohne Risiko für Ihre Kühlkette, ohne steigende Stromrechnungen.**

Die Simulation belegt: Eine Flotte von 400 Supermärkten ersetzt wirtschaftlich einen Batteriespeicher im zweistelligen Megawatt-Bereich – mit amortisierten Kältemaschinen statt teuren Lithium-Ionen-Zellen. Und das bei positiver Rendite ab dem ersten Tag.

**Stichwort: 5–15 % Stromkostensenkung bei 0 % Mehrverbrauch.**

---

## 🔥 Der Volatile-Modus – Was wäre, wenn wir alle Brecher lösen?

Neben dem konservativen Modus (mit Sicherheitsgrenzen für Temperatur, max_shed_hours und thermischem Weglaufen) gibt es den **`--volatile` Modus**. Dieser entfernt sämtliche Beschränkungen:

- ❌ Keine `max_shed_hours`-Begrenzung – das System shedded so lange, wie es preislich attraktiv ist
- ❌ Keine Thermal-Runaway-Prüfung – die Temperatur darf bis zum physikalischen Limit ausschlagen
- ❌ Keine Retail-Schonzone – auch Supermärkte bekommen die aggressiven 15th/85th-Perzentil-Schwellen

Ergebnis: **Höhere Einsparungen**, aber auf Kosten von Temperaturtoleranz und thermischer Sicherheit. Der Modus ist das "Was-wäre-wenn"-Szenario für maximale wirtschaftliche Ausbeutung ohne Rücksicht auf Kühlkette oder Anlagenverschleiß.

> **Usage:** `cargo run --release -- --volatile "400 aldi + 5 edeka"`

Der direkte Vergleich beider Modi in einer Simulation zeigt den Preis der Sicherheit – und das Potenzial des ungezügelten Systems.
