# HoloForge — SPRIND Deep-Tech Bootcamp & Interview-Leitfaden

**Kompendium für das Erstgespräch, Physik-Audits & Due Diligence bei der SPRIND**  
*Projekt: HoloForge (Tomo-H-VAM) • Einreicher: Lukas Negenborn (Keller Systems) • Stand: November 2026*

---

## 1. Dein 60-Sekunden-Eröffnungs-Pitch am Telefon

Wenn der SPRIND-Scout oder Challenge Officer fragt: *„Herr Negenborn, erzählen Sie uns doch mal kurz in eigenen Worten, was HoloForge macht und wo Sie gerade stehen“*, ist das dein Leitfaden:

> „Sehr gerne. Klassischer 3D-Druck baut Teile stundenlang Schicht für Schicht auf – mit Stützstrukturen, rauen Stufen und Sollbruchstellen zwischen den Schichten. HoloForge dreht das Prinzip der medizinischen Computertomographie (CT) um: Wir projizieren kohärentes 405-Nanometer-Laserlicht über einen Phasen-SLM (Spatial Light Modulator) aus verschiedenen Winkeln in ein rotierendes Harz-Gefäß. Ein einzelner Winkel hat zu wenig Energie zum Härten. Aber dort im 3D-Raum, wo sich die Wellenfronten aus allen Winkeln überlagern, wird die chemische Schwelle überschritten und das gesamte Bauteil erstarrt simultan in unter 15 Sekunden – komplett ohne Stützstrukturen und mit optisch glatten Oberflächen.
>
> Aktuell steht der komplette mathematisch-physikalische Digitale Zwilling als Software-Stack (`holoforge-opt`): Wellenoptik, Polymerisationskinetik, Gerätetreiber und unsere analytische Zylinderglas-Aberrationskorrektur, durch die wir kein giftiges Immersionsölbad mehr brauchen. Mit der SPRIND-Unterstützung will ich jetzt den Schritt aus der verifizierten Simulation auf die physische optische Bank machen.“

---

## 2. Die 5 physikalischen Kernsäulen (Was unter der Haube passiert)

### Säule 1: Warum ein Phasen-SLM statt eines normalen Beamers (DLP)?

* **Einfach erklärt:**  
  Andere Forscher (wie in Berkeley beim CAL-Verfahren) nehmen einen normalen Kino-Beamer (DLP-Chip mit kleinen Kippspiegeln). Wenn ein Beamer ein dunkles Pixel machen will, kippt er den Spiegel weg und **vernichtet 50 bis 80 % des Laserlichts als Wärme**. Außerdem wird das Bild in der Tiefe der Flüssigkeit schnell unscharf wie bei einem normalen Projektor.  
  HoloForge nutzt einen **Phasen-SLM** (*Liquid Crystal on Silicon*). Der blockiert kein Licht, sondern bremst die Lichtwelle an jedem Pixel minimal ab (verschiebt die Phase von $0$ bis $2\pi$). Dadurch lenkt er **$> 90\%$ der Lichtenergie** nahezu verlustfrei genau dorthin, wo sie im 3D-Raum gebraucht wird – und kann den Fokus elektronisch tief in das Glas hineinprogrammieren.
* **Deine Fach-Antwort im Call:**  
  *„Phasenmodulation statt Amplitudenmodulation: Wir nutzen LCoS-Phasen-SLMs mit $8{,}0\,\mu\text{m}$ Pixel-Pitch. Über eine telezentrische $4f$-Optik ($f_1 = 200\text{ mm}, f_2 = 50\text{ mm}$) verkleinern wir das Feld um den Faktor $M = 0{,}25\times$ und erreichen im Harz eine effektive Voxel-Auflösung von $p_{\text{vat}} = 2{,}0\,\mu\text{m}$.“*

### Säule 2: Dein größter Trumpf – Die Zylinder-Korrektur (Kein Ölbad!)

Das ist dein **wichtigster USP** im Gespräch:

* **Das Problem:**  
  Das flüssige Harz befindet sich in einem runden Glasröhrchen (weil es sich dreht). Wenn Licht auf ein rundes Glas trifft, wirkt das runde Glas wie eine starke Lupe (Zylinderlinse) und bricht die Strahlen krumm (starker Astigmatismus).
* **Wie die Konkurrenz (Readily3D / Berkeley) das löst:**  
  Sie stellen das runde Glasröhrchen in ein zweites, viereckiges Aquarium voll mit einer öligen Chemikalie (*Index-Matching-Bath*), die denselben Brechungsindex hat wie das Glas. Das ist unhandlich, teuer, oft toxisch und für eine Zahnarztpraxis unbrauchbar.
* **Wie HoloForge das löst (`cylindrical_vat_phase_correction` in `optics.py`):**  
  Du kennst den Außen- und Innenradius des Glases ($R_{\text{out}}, R_{\text{in}}$) und die Brechungsindizes von Glas ($n_{\text{glas}} \approx 1{,}51$) und Harz ($n_{\text{harz}} \approx 1{,}46$).  
  Dein Algorithmus berechnet die Wölbung des Glases (*Sagitta* / Pfeilhöhe):
  $$\text{sag}(x) = R - \sqrt{R^2 - x^2}$$
  Daraus bestimmt er den optischen Gangunterschied ($\text{OPD}$):
  $$\text{OPD}(x) = (n_{\text{glas}} - 1)\,\text{sag}_{\text{out}}(x) - (n_{\text{glas}} - n_{\text{harz}})\,\text{sag}_{\text{in}}(x)$$
  und legt **das exakte Gegenteil (das inverse Phasenkonjugat $-\phi_{\text{vat}} = -\frac{2\pi}{\lambda_0}\text{OPD}$)** direkt auf den SLM-Chip.  
  **Bildhaft:** Du schickst das Licht absichtlich genau so „vor-verkrümmt“ los, dass das runde Glas es beim Durchgehen wieder **schnurgerade** biegt! Kein Ölbad nötig – das Glas dreht sich trocken an der Luft.

### Säule 3: Warum wird nicht alles zu einem einzigen Klumpen? (Die Sauerstoff-Inhibition)

Ein Gutachter wird garantiert fragen: *„Wenn Sie von allen Seiten Licht durch das Glas schießen, bekommt doch auch das Harz am Rand ständig etwas Licht ab. Warum härtet das Harz auf dem Weg zur Mitte nicht auch aus?“*

* **Die Antwort (`kinetics.py`):**  
  Ohne Sauerstoff würde tatsächlich alles verschmieren (lineare Optik). Aber in unserem Harz (PEGDA / HDDA mit $1\text{ wt\%}$ TPO-L Photoinitiator) ist **molekularer Sauerstoff ($\text{O}_2$) gelöst**.
  1. Trifft 405-nm-Licht auf den Photoinitiator, entstehen freie Radikale ($R^\bullet$).
  2. Solange Sauerstoff an dieser Stelle vorhanden ist, schnappt sich der Sauerstoff das Radikal **1.000.000-mal schneller** als das Harz vernetzen kann ($k_{\text{inh}} \approx 10^9\text{ M}^{-1}\text{s}^{-1}$ gegenüber $k_p \approx 10^3\text{ M}^{-1}\text{s}^{-1}$) und bildet inaktive Peroxyradikale ($ROO^\bullet$). **Es passiert exakt null Härtung!**
  3. **Erst an den Schnittpunkten im Bauteil**, wo so viel Licht aus allen Winkeln zusammenkommt, dass der lokale Sauerstoff **komplett aufgebraucht** ist ($[\text{O}_2] < [\text{O}_2]_{\text{crit}}$), fällt die Schutzmauer. Sobald die Gelierungsschwelle ($D_{\text{gel}} \approx 10\text{–}18\text{ mJ/cm}^2$) erreicht ist, vernetzt das Polymer schlagartig in unter einer Sekunde!
  * **Dein Fach-Begriff:** *„Nichtlineare Heaviside-Schwellenwert-Kinetik durch radikalische Sauerstoff-Inhibition.“*

### Säule 4: Wie rechnet deine Optik-Engine? (`optics.py` & `optimize.py`)

Falls ein Physiker oder Mathematiker fragt, wie dein Simulator das Licht berechnet:

1. **Split-Step Angular Spectrum Method (ASM):**  
   Wir teilen das Harzvolumen in dünne Scheiben ($\Delta z$). Für jede Scheibe rechnen wir:
   * Halber Schritt freie Wellenausbreitung im Frequenzraum (FFT):
     $$H_{\text{half}}(f_x, f_y) = \exp\left(i\frac{\Delta z}{2}\sqrt{k^2 - (2\pi f_x)^2 - (2\pi f_y)^2}\right)$$
   * Zurück in den Ortsraum (IFFT): Faltung mit einem Gauß-Kern für die **Vorwärtsstreuung** ($\theta_{\text{rms}} = \sqrt{p / L_{\text{scatter}}}$) und Amplitudendämpfung nach **Beer-Lambert** ($\exp(-\frac{1}{2}\alpha \Delta z)$).
   * Zweiter halber Schritt im Frequenzraum.
2. **Band-Decomposed Multi-Plane Gerchberg-Saxton (MP-GS):**  
   Weil ein optisches System eine endliche Tiefenschärfe (Rayleigh-Länge $z_R = \frac{\lambda}{\pi\,\text{NA}^2}$) hat, teilen wir das 3D-Objekt in $K$ Tiefenbänder auf.
   * **Least-Squares-Skalierung ($s = \frac{\operatorname{Re}\langle D, U \rangle}{\|D\|_2^2}$):** Passt die Zielamplitude optimal an das feste Energiebudget des Lasers an, damit der Optimierer nicht stagniert.
   * **Guard Slices (Wächter-Schichten):** Direkt vor und hinter jedem Zielband dämpfen wir jedes Streulicht im Algorithmus mit dem Faktor $0{,}25$, um Überbluten (*Out-of-band bleed*) zu verhindern.
   * **Exakter Adjoint Backward Walk:** Projektion des korrigierten Feldes zurück zur SLM-Ebene über den adjungierten Operator $\mathcal{P}^\dagger$.
   * **Iterative Dose Compensation:** Eine äußere Regelschleife misst den 3D-Dosisfehler und verstärkt tiefer liegende, durch Absorption geschwächte Voxel automatisch vor.

### Säule 5: Warum hat das System ZWEI Lichtfarben (405 nm Violett + 635 nm Rot)?

* **405 nm (Violetter Laser):** Ist das „Arbeitslicht“. Darauf reagiert der Photoinitiator (TPO-L) und härtet das Harz.
* **635 nm (Rotes Licht + Schlieren-Kamera):** Ist das „Überwachungsauge“. Das Harz ist für rotes Licht **komplett blind** – rotes Licht härtet nichts aus!  
  Aber: Sobald das Harz in der Mitte geliert (fest wird), ändert sich seine Dichte und sein **Brechungsindex springt um $\Delta n \approx 0{,}02\text{ bis }0{,}05$**. Das rote Licht wird an dieser Kante gebrochen (Schlieren-Effekt). Die Kamera erkennt diesen Phasenübergang in Echtzeit und schaltet den 405-nm-Laser exakt am Gelierungspunkt ab (**Closed-Loop Endpoint Detection**).

---

## 3. Die 7 härtesten Grill-Fragen der Gutachter (und deine Antworten)

### Frage 1: *„Haben Sie das schon physisch im Labor gedruckt oder ist alles bisher nur auf dem Computer simuliert?“*
* **Grundregel:** Niemals flunkern!
* **Deine Antwort:**  
  *„Nein, bisher ist das zu 100 % in unserem gekoppelten Digitalen Zwilling simuliert – und genau deshalb habe ich das im Whitepaper auch ganz oben in einer eigenen Box unter ‚Data Provenance‘ transparent deklariert. Die 92,3 % Cure-IoU und die <0,02 % Overcure stammen aus dem Simulator mit gepinntem Zufalls-Seed (jederzeit reproduzierbar über `python -m holoforge.cli verify`). Genau für den Übergang von TRL 3 (validierte Simulation) zu TRL 4 (physische optische Bank) habe ich das Projekt bei SPRIND eingereicht.“*

### Frage 2: *„Ein kohärenter Laser erzeugt doch ‚Speckle‘ (Laser-Körnung durch Interferenz). Macht das Ihr Bauteil nicht löchrig wie Schweizer Käse?“*
* **Deine Antwort:**  
  *„Absolut wichtiger Punkt! Bei einem einzelnen statischen Phasen-Hologramm hätte man starkes Speckle-Rauschen. Wir unterdrücken das durch zwei Effekte: Erstens über **zeitliches und winkelabhängiges Multiplexing** von $K$ verschiedenen Phasenmustern während der Rotation – die unkorrelierten Speckle-Kontraste mitteln sich mit $1/\sqrt{K}$ heraus. Zweitens wirkt die **Diffusionslänge der freien Radikale und des Sauerstoffs** auf der Skala von $1\text{–}2\,\mu\text{m}$ als natürlicher chemischer Tiefpassfilter, der hochfrequente Intensitätsspitzen glättet.“*

### Frage 3: *„Was passiert, wenn das Teil in der Flüssigkeit fest wird – sinkt es dann nicht durch die Schwerkraft nach unten weg, bevor es fertig ist?“*
* **Deine Antwort:**  
  *„Nein, aus zwei Gründen: Erstens arbeiten wir mit angepasster Harz-Viskosität ($0{,}15\text{–}0{,}65\text{ Pa}\cdot\text{s}$), sodass die Stokes-Sinkgeschwindigkeit über das kurze Zeitfenster minimal ist. Zweitens verläuft die Belichtung während der ersten 10–12 Sekunden in der Sauerstoff-Inhibitionsphase (Induktionsphase), in der das Harz überall flüssig bleibt. Das gesamte Zielvolumen erreicht fast synchron den Gelierungspunkt $D_{\text{gel}}$. Der eigentliche Phasenübergang von flüssig zu fest passiert dann im letzten Bruchteil einer Sekunde schlagartig im gesamten Volumen.“*

### Frage 4: *„Bei schneller Polymerisation entsteht Wärme (Exothermie). Führt das nicht zu Schlieren und Konvektionsströmungen im Glas?“*
* **Deine Antwort:**  
  *„Genau das habe ich als zentrales Risiko im Antrag adressiert. Wenn man zu viel Photoinitiator nutzt und ungetaktet feuert, entsteht thermische Konvektion. Deshalb halten wir die TPO-L-Konzentration niedrig ($\approx 0{,}5\text{–}1{,}0\text{ wt\%}$, Absorptionskoeffizient $\alpha \approx 0{,}2\text{–}0{,}6\text{ mm}^{-1}$), verteilen die Energie homogen aus 360° und steuern die Laserleistung über getaktete Dosisrampen, sodass das Teil erstarrt, bevor thermischer Auftrieb einsetzen kann.“*

### Frage 5: *„In Berlin gibt es schon ‚xolo 3D‘ (Xolographie) und in der Schweiz ‚Readily3D‘. Warum brauchen wir HoloForge?“*
* **Deine Antwort:**  
  *„Beide nutzen grundlegend andere Verfahren mit harten Einschränkungen:*
  * *xolo 3D (Xolographie) benötigt zwei gekreuzte Wellenlängen und extrem teure, proprietäre Dual-Color-Photoswitch-Initiatoren, die kaum kommerziell oder biokompatibel verfügbar sind. Zudem fährt dort ein Lichtblatt mechanisch langsam durch die Küvette.*
  * *Readily3D nutzt klassische DLP-Amplitudenprojektion (verwirft $>60\%$ der Lichtenergie) und benötigt ein flüssiges Brechungsindex-Anpassungsbad.*
  * *HoloForge arbeitet mit **Standard-405-nm-Industrieharzen**, nutzt **verlustfreie Phasen-Holographie (>90 % Effizienz)** und kommt durch unser inverses Phasenkonjugat **komplett ohne flüssiges Immersionsbad** aus.“*

### Frage 6: *„Wie sieht Ihr allererster physischer Test aus, wenn Sie das Geld für Arbeitspaket 1 erhalten?“*
* **Deine Antwort (Aus deiner `BOM.md`):**  
  *„Wir gehen streng zweistufig vor: In Stufe 1 füllen wir noch kein Harz in die Quarzküvette, sondern eine **Fluoreszenz-Lösung** (Rhodamin 6G oder Natrium-Fluoreszein in Ethanol). Wir projizieren unsere berechneten Phasen-Hologramme inklusive Zylinderkorrektur hinein und vermessen mit einer orthogonalen CMOS-Kamera das 3D-Strahlprofil im Glas (Ziel: Peak-to-Background-Kontrast $> 8:1$). Erst wenn die Wellenfront optisch im Zylinder verifiziert ist, wechseln wir in Stufe 2 auf das PEGDA/TPO-L-Harz und härten die ersten 10-mm-Prüfkörper.“*

### Frage 7: *„Sie machen aktuell eine Ausbildung zum Fachinformatiker bei fetra Fechtel. Wie wollen Sie das Projekt zeitlich stemmen?“*
* **Deine Antwort:**  
  *„Den gesamten bisherigen Mathematik-, Wellenoptik- und Software-Stack von HoloForge habe ich ebenfalls hochfokussiert parallel aufgebaut. Die gesamte algorithmische Vorarbeit (Digitaler Zwilling, Treiber-Schicht, Optimierer) steht bereits fertig getestet. In der Validierungsphase geht es jetzt um den gezielten Aufbau des optomechanischen Prüfstands. Zudem habe ich im Antrag bereits angegeben, dass ich über das SPRIND-Netzwerk gezielt nach einem Co-Founder oder Laborpartner aus der Optomechanik/Photonik suche, um die Laborversuche operativ vor Ort zu skalieren.“*

---

## 4. Alle Zahlen & Fakten auf einen Blick (Spickzettel)

| Parameter | Wert in HoloForge | Warum dieser Wert? |
| :--- | :--- | :--- |
| **Wellenlänge Härten ($\lambda_0$)** | $405\text{ nm}$ (Violett) | Standard-Absorptionsbande für industrielle Photoinitiatoren (TPO-L / BAPO). |
| **Wellenlänge Kamera** | $635\text{ nm}$ (Rot) | Harz ist bei $635\text{ nm}$ völlig unempfindlich; ideal für Schlieren-Überwachung. |
| **SLM-Auflösung & Pitch** | $1920 \times 1080$, $8{,}0\,\mu\text{m}$ Pitch | LCoS Phasen-SLM (z. B. Holoeye Pluto-2.1), Phasen-Hub $0\text{ bis }2\pi$. |
| **Optische Verkleinerung ($4f$)** | $M = 0{,}25\times$ ($f_1 = 200\text{ mm}, f_2 = 50\text{ mm}$) | Verkleinert das $8\,\mu\text{m}$ SLM-Pixel auf **$2{,}0\,\mu\text{m}$ Voxel-Pitch** im Harz. |
| **Simulierte Formtreue (Cure IoU)** | **$92{,}3\,\%$** (Seed 7) | Überlappung zwischen Ziel-Geometrie und gehärtetem Volumen (`holoforge verify`). |
| **Overcure (Überhärtung)** | **$< 0{,}02\,\%$** | Nahezu kein ungewolltes Aushärten außerhalb der Bauteilgrenzen. |
| **Druckzeit** | **$10\text{ bis }15\text{ Sekunden}$** | Inklusive Sauerstoff-Induktionsphase (~10 s) + Vernetzung (<5 s). |
| **Harz-Modell** | PEGDA / HDDA + $1{,}0\text{ wt\%}$ TPO-L | Gelierungsschwelle $D_{\text{gel}} \approx 10\text{–}18\text{ mJ/cm}^2$, Brechungsindex $n \approx 1{,}46\text{–}1{,}52$. |
| **Hardware-Kosten Prüfstand (BOM)** | **ca. 8.300 € – 11.800 €** | SLM (~6.000 €), 405nm Laser (~900 €), Thorlabs $4f$-Optik & Käfig (~2.000 €), Kamera (~500 €). |
| **Gesamtbudget im Antrag** | **85.000 € über 9 Monate** | AP1 Prüfstand (28k), AP2 Chemie (18k), AP3 Closed-Loop (21k), AP4 Prüfkörper (18k). |

---

## 5. Dein Live-Demo-Trumpf für ein Video-Interview

Falls SPRIND dich zu einem **Zoom- oder Teams-Call** einlädt, startest du vor dem Gespräch im Ordner `C:\Users\lukas\Downloads\SYSTEMS & CREATIONS\HoloForge` dein Dashboard:

```powershell
python -m holoforge.cli ui --port 8080
```

Dann öffnest du `http://127.0.0.1:8080` im Browser, teilst im Video-Call deinen Bildschirm, wählst live eine Zahnkrone (`dental`), eine Linse (`lens`) oder einen Mikrofluidik-Chip (`microfluidics`) aus, stellst den Glasradius ein und klickst auf **„Synthesize Tomographic Holograms“**.  
Damit zeigst du den Gutachtern live in Echtzeit die berechneten SLM-Phasenmasken, die 3D-Voxel-Wolke und das simulierte 635-nm-Schlierenbild.
