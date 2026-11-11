/**
 * BauLens Legal Knowledge Base
 * Verbatim statutory texts, legal commentaries, and exemption guidance
 * across Federal laws (BauGB, BauNVO), State codes (MBO, BayBO, LBO BW, etc.),
 * and Municipal bylaws.
 */

export const STATUTES_DB = {
  // 1. BauNVO & Federal Law
  "baunvo_19": {
    id: "baunvo_19",
    norm: "§ 19 BauNVO",
    title: "Grundflächenzahl (GRZ) und zulässige Grundfläche",
    jurisdiction: "Bundesrecht (BauNVO)",
    wortlaut: `(1) Die Grundflächenzahl gibt an, wieviel Quadratmeter Grundfläche je Quadratmeter Grundstücksfläche im Sinne des § 19 Abs. 3 zulässig sind.
(2) Zulässige Grundfläche ist der nach Absatz 1 errechnete Anteil des Baugrundstücks, der von baulichen Anlagen überdeckt werden darf.
(4) Bei der Ermittlung der Grundfläche sind die Grundflächen von
1. Garagen und Stellplätzen mit ihren Zufahrten,
2. Nebenanlagen im Sinne des § 14,
3. baulichen Anlagen unterhalb der Geländeoberfläche, durch die das Baugrundstück lediglich unterbaut wird,
mitzurechnen. Die zulässige Grundfläche darf durch die Grundflächen der in Satz 1 bezeichneten Anlagen bis zu 50 vom Hundert überschritten werden, höchstens jedoch bis zu einer Grundflächenzahl von 0,8.`,
    commentary: `Die GRZ sichert die städtebauliche Dichte und den Grünflächenerhalt. Das Überschreitungsprivileg nach Abs. 4 erlaubt für Nebenanlagen und Stellplätze einen Aufschlag von bis zu 50%, solange der Bebauungsplan dies nicht explizit ausschließt.`,
    remedy: `Wird die GRZ I (Hauptgebäude) oder GRZ II (inkl. Zufahrten) überschritten, muss ein Befreiungsantrag nach § 31 Abs. 2 BauGB mit städtebaulicher Begründung (Würdigung nachbarlicher Interessen) gestellt werden.`
  },
  "baunvo_20": {
    id: "baunvo_20",
    norm: "§ 20 BauNVO",
    title: "Vollgeschosse, Geschossflächenzahl (GFZ), Geschossfläche",
    jurisdiction: "Bundesrecht (BauNVO)",
    wortlaut: `(1) Als Vollgeschosse gelten Geschosse, die nach landesrechtlichen Vorschriften Vollgeschosse sind oder auf ihre Zahl angerechnet werden.
(2) Die Geschossflächenzahl gibt an, wieviel Quadratmeter Geschossfläche je Quadratmeter Grundstücksfläche zulässig sind.
(3) Die Geschossfläche ist nach den Außenmaßen der Gebäude in allen Vollgeschossen zu ermitteln. Die Flächen von Aufenthaltsräumen in anderen Geschossen einschließlich der zu ihnen gehörenden Treppenräume und einschließlich ihrer Umfassungswände sind mitzurechnen.`,
    commentary: `Die GFZ regelt das bauliche Maß der Nutzung im Raum. Im Gegensatz zur GRZ ist eine Überschreitung der GFZ im Baugenehmigungsverfahren ein schwerer formeller Mangel, der selten durch Befreiung geheilt werden kann.`,
    remedy: `Flächenreduktion der Vollgeschosse oder Umplanung von Vollgeschossen in Nicht-Vollgeschosse (z. B. Staffelgeschoss nach Landesbauordnung) vor Bauantragseinreichung.`
  },
  "baunvo_16": {
    id: "baunvo_16",
    norm: "§ 16 BauNVO",
    title: "Bestimmung des Maßes der baulichen Nutzung",
    jurisdiction: "Bundesrecht (BauNVO)",
    wortlaut: `(1) Bei der Festsetzung des Maßes der baulichen Nutzung im Bebauungsplan werden stets bestimmt
1. die Höhe baulicher Anlagen oder
2. die Zahl der Vollgeschosse oder
3. die Baumassenzahl oder
4. die Grundflächenzahl oder die Geschossflächenzahl.
(2) Festsetzungen nach Absatz 1 können miteinander verbunden werden.`,
    commentary: `Dient der harmonischen Einfügung in das Ortsbild und der Verhinderung städtebaulicher Überlastung der Infrastruktur.`,
    remedy: `Plausibilitätsprüfung zwischen Gebäudehöhe und Vollgeschossanzahl vornehmen.`
  },

  // 2. MBO (Musterbauordnung) & Standard Landesbauordnungen
  "mbo_2": {
    id: "mbo_2",
    norm: "§ 2 MBO",
    title: "Begriffe und Einstufung in Gebäudeklassen",
    jurisdiction: "Musterbauordnung / Landesbauordnungen",
    wortlaut: `(3) Gebäude werden in folgende Gebäudeklassen eingeteilt:
1. Gebäudeklasse 1: freistehende Gebäude mit einer Höhe bis zu 7 m und nicht mehr als zwei Nutzungseinheiten von insgesamt nicht mehr als 400 m²,
2. Gebäudeklasse 2: Gebäude mit einer Höhe bis zu 7 m und nicht mehr als zwei Nutzungseinheiten von insgesamt nicht mehr als 400 m²,
3. Gebäudeklasse 3: sonstige Gebäude mit einer Höhe bis zu 7 m,
4. Gebäudeklasse 4: Gebäude mit einer Höhe bis zu 13 m und Nutzungseinheiten mit jeweils nicht mehr als 400 m²,
5. Gebäudeklasse 5: sonstige Gebäude einschließlich unterirdischer Gebäude.
Höhe ist das Maß der Fußbodenoberkante des höchstgelegenen Geschosses, in dem ein Aufenthaltsraum möglich ist, über der Geländeoberfläche im Mittel.`,
    commentary: `Die Gebäudeklasse determiniert die materiellen Anforderungen an den Brandschutz (Feuerwiderstand der tragenden Wände, Decken und Treppenhausqualitäten von F30 bis F90-AB).`,
    remedy: `Einstufung prüfen; durch geringe Absenkung der Oberkante des obersten Aufenthaltsgeschosses unter 13,00 m kann der Wechsel von GK 4 zu GK 3 erreicht werden, was erhebliche Baukostenvorteile bringt.`
  },
  "mbo_6": {
    id: "mbo_6",
    norm: "§ 6 MBO",
    title: "Abstandsflächen und Abstände",
    jurisdiction: "Musterbauordnung / Standard-LBO",
    wortlaut: `(1) Vor den Außenwänden von Gebäuden sind Abstandsflächen freizuhalten. Die Abstandsflächen müssen auf dem Grundstück selbst liegen.
(5) Die Tiefe der Abstandsfläche beträgt 0,4 H, mindestens jedoch 3 m. In Gewerbe- und Industriegebieten genügt eine Tiefe von 0,2 H, mindestens 3 m.
(6) Vor zwei Außenwänden eines Gebäudes von nicht mehr als 16 m Länge genügt als Tiefe der Abstandsfläche 0,2 H, mindestens 3 m (Schmalseitenprivileg - landesspezifisch).`,
    commentary: `Dient der Belichtung, Belüftung, dem Brandschutz und dem sozialen Nachbarschutz. Maßgeblich ist die Wandhöhe H bis zum Schnittpunkt mit der Dachhaut bzw. zzgl. Dachanteilen bei steilen Neigungen.`,
    remedy: `Bei Überschreitung der Grundstücksgrenze muss entweder eine Baulast / Abstandsflächenübernahme auf dem Nachbargrundstück (§ 7 MBO) dinglich gesichert oder das Gebäude abgerückt werden.`
  },
  "lbo_bw_6": {
    id: "lbo_bw_6",
    norm: "§ 6 LBO BW",
    title: "Abstandsflächen in Baden-Württemberg",
    jurisdiction: "Landesrecht Baden-Württemberg",
    wortlaut: `(1) Vor den Außenwänden von Gebäuden müssen Abstandsflächen liegen, die von oberirdischen baulichen Anlagen freizuhalten sind.
(5) Die Tiefe der Abstandsfläche bemisst sich nach der Wandhöhe (H). Sie beträgt im Allgemeinen 0,5 H, in Kern-, Gewerbe- und Industriegebieten 0,25 H. Die Tiefe darf jedoch 5 m, in Gewerbe- und Industriegebieten 3 m nicht unterschreiten.`,
    commentary: `Baden-Württemberg besitzt im bundesweiten Vergleich mit einem Mindestabstandsmaß von 5,00 m (anstelle der in MBO/BayBO/NRW üblichen 3,00 m) eines der strengsten Abstandsflächengesetze für den Wohnbau.`,
    remedy: `Planung zwingend auf 5,00 m Grenzabstand ausrichten oder Befreiung / Baulast nach § 7 LBO BW einholen.`
  },
  "baybo_22": {
    id: "baybo_22",
    norm: "§ 22 BayBO",
    title: "Bebauung der Grundstücke und Grenzbebauung in Bayern",
    jurisdiction: "Landesrecht Bayern",
    wortlaut: `(1) Bauliche Anlagen sind so anzuordnen, dass sie sich in die Eigenart der näheren Umgebung einfügen und das Ortsbild nicht beeinträchtigen.
(2) Gebäude dürfen an die Grundstücksgrenze gebaut werden, wenn nach städtebaulichem Recht (Bebauungsplan oder § 34 BauGB) geschlossen gebaut werden muss oder darf, oder wenn die Grenzbebauung mit der nachbarlichen Bebauung übereinstimmt.`,
    commentary: `Die Bayerische Bauordnung setzt prioritär auf das Einfügungsgebot und die Bebauungsplanfestsetzungen. Grenzbebauung ist stark an das Nachbarschafts- und Baulinienregime gekoppelt.`,
    remedy: `Prüfung des qualifizierten Bebauungsplans oder bauplanungsrechtlichen Vorbescheid nach Art. 71 BayBO einreichen.`
  },
  "bauo_nrw_6": {
    id: "bauo_nrw_6",
    norm: "§ 6 BauO NRW",
    title: "Abstandsflächen in Nordrhein-Westfalen (0,4 H)",
    jurisdiction: "Landesrecht Nordrhein-Westfalen",
    wortlaut: `(1) Vor den Außenwänden von Gebäuden sind Abstandsflächen freizuhalten.
(5) Die Tiefe der Abstandsfläche beträgt 0,4 H, mindestens jedoch 3 m. In Gewerbe- und Industriegebieten genügt eine Tiefe von 0,2 H, mindestens jedoch 3 m.
(8) In den Abstandsflächen eines Gebäudes sowie ohne eigene Abstandsflächen sind an der Grundstücksgrenze Garagen und überdachte Stellplätze mit einer mittleren Wandhöhe bis zu 3 m und einer Gesamtlänge je Grundstücksgrenze von 9 m zulässig; die Gesamtlänge aller Grenzbebauungen darf 15 m nicht überschreiten.`,
    commentary: `NRW hat im Rahmen der Baurechtsmodernisierung das Abstandsflächenmaß von ehemals 0,8 H / 0,5 H auf bundesweit moderne 0,4 H gesenkt. Das Grenzgaragenprivileg erlaubt bis zu 9 m Grenzbebauung je Grenze (max. 15 m insgesamt).`,
    remedy: `0,4 H einhalten; bei Unterschreitung Grenzgarage deklarieren oder Abstandsflächenbaulast nach § 85 BauO NRW prüfen.`
  },
  "hbo_6": {
    id: "hbo_6",
    norm: "§ 6 HBO",
    title: "Abstandsflächen in Hessen (0,4 H)",
    jurisdiction: "Landesrecht Hessen",
    wortlaut: `(1) Vor den Außenwänden von oberirdischen Gebäuden sind Abstandsflächen freizuhalten.
(5) Die Tiefe der Abstandsfläche bemisst sich nach der Wandhöhe (H). Sie beträgt 0,4 H, mindestens 3 m. In Gewerbe-, Industrie- und Sondergebieten genügt 0,2 H, mindestens 3 m.`,
    commentary: `Die Hessische Bauordnung verlangt 0,4 H für allgemeine Wohngebiete und eröffnet zügige Nachverdichtungsmöglichkeiten.`,
    remedy: `Grenzabstand mit 0,4 H abgleichen oder Abweichung nach § 73 HBO beantragen.`
  },
  "nbauo_5": {
    id: "nbauo_5",
    norm: "§ 5 NBauO",
    title: "Grenzabstand und Abstandsflächen in Niedersachsen",
    jurisdiction: "Landesrecht Niedersachsen",
    wortlaut: `(1) Gebäude müssen mit ihren Außenwänden einen Mindestabstand von 3 m von den Grenzen des Baugrundstücks einhalten.
(2) Der Abstand bemisst sich nach der Höhe der Außenwand (H). Er beträgt im Allgemeinen 0,5 H, mindestens jedoch 3 m.`,
    commentary: `Niedersachsen hält grundsätzlich an 0,5 H als Bemessungsmaßstab fest, privilegiert jedoch Nebenanlagen und Grenzgaragen bis 9 m Grenzlänge.`,
    remedy: `0,5 H (mind. 3 m) einhalten oder Grenzabstandsübernahme im Baulastenverzeichnis sichern.`
  },
  "hbauo_6": {
    id: "hbauo_6",
    norm: "§ 6 HBauO",
    title: "Abstandsflächen in Hamburg (0,4 H)",
    jurisdiction: "Landesrecht Hamburg",
    wortlaut: `(1) Vor den Außenwänden von Gebäuden sind Abstandsflächen freizuhalten.
(5) Die Tiefe der Abstandsfläche beträgt 0,4 H, mindestens jedoch 3 m.`,
    commentary: `Hamburg setzt für urbane Nachverdichtung einheitlich auf 0,4 H bei 3,0 m Mindesttiefe.`,
    remedy: `Einhaltung von 0,4 H sicherstellen.`
  },
  "mbo_46": {
    id: "mbo_46",
    norm: "§ 46 MBO",
    title: "Aufenthaltsräume und Lichte Raumhöhe",
    jurisdiction: "Musterbauordnung / Landesbauordnungen",
    wortlaut: `(1) Aufenthaltsräume müssen eine lichte Raumhöhe von mindestens 2,50 m, im Dachraum über mindestens der Hälfte ihrer Grundfläche mindestens 2,30 m haben.
(2) Für Aufenthaltsräume in Wohngebäuden der Gebäudeklassen 1 und 2 kann eine lichte Höhe von mindestens 2,40 m gestattet werden.`,
    commentary: `Sichert die physiologische und hygienische Mindestqualität für Wohn- und Arbeitsräume. Raumhöhen unter 2,20 m führen regelmäßig zur Versagung der Aufenthaltsraum-Eigenschaft.`,
    remedy: `Bei Rohbaumaßen den Fußbodenaufbau (Estrich + Trittschalldämmung + Belag ~ 14–18 cm) exakt einrechnen, um die lichte Fertighöhe von mind. 2,40 m bzw. 2,50 m zu garantieren.`
  },
  "mbo_47": {
    id: "mbo_47",
    norm: "§ 47 MBO / DIN 5034",
    title: "Belichtung und Belüftung (1/8-Fensterflächenregel)",
    jurisdiction: "Musterbauordnung & DIN 5034",
    wortlaut: `(1) Aufenthaltsräume müssen ausreichend durch Tageslicht belichtet werden können und belüftet werden können.
(2) Das Rohbaumaß der Fensteröffnungen muss mindestens ein Achtel (12,5%) der Netto-Grundfläche des Raumes betragen. Dachflächenfenster und verglaste Türen werden angerechnet.`,
    commentary: `Die 1/8-Regel ist die bauaufsichtliche Basisanforderung für gesunde Wohnverhältnisse. Räume mit weniger als 12,5% Fensterfläche gelten nicht als genehmigungsfähige Aufenthaltsräume.`,
    remedy: `Vergrößerung der Fensteröffnungen oder zusätzliche Oberlichter / Lichtbänder einplanen.`
  },
  "mbo_14": {
    id: "mbo_14",
    norm: "§ 14 & § 33 MBO",
    title: "Brandschutz, Erster und Zweiter Rettungsweg",
    jurisdiction: "Musterbauordnung / Landesbauordnungen",
    wortlaut: `§ 14: Bauliche Anlagen sind so anzuordnen, zu errichten, zu ändern und instand zu halten, dass der Entstehung eines Brandes und der Ausbreitung von Feuer und Rauch vorgebeugt wird und bei einem Brand die Rettung von Menschen und Tieren sowie wirksame Löscharbeiten möglich sind.
§ 33: Für Nutzungseinheiten mit mindestens einem Aufenthaltsraum müssen in jedem Geschoss mindestens zwei voneinander unabhängige Rettungswege ins Freie vorhanden sein. Der erste Rettungsweg muss über eine notwendige Treppe führen. Der zweite Rettungsweg kann eine mit Rettungsgeräten der Feuerwehr erreichbare Stelle sein.`,
    commentary: `Ab Gebäudeklasse 4 ist ein eigener baulicher Treppenraum (F90 Wände, T30 Türen) gesetzlich zwingend. Bei fehlendem Treppenraum oder unzureichender Anleiterbarkeit durch die Feuerwehr droht Baustopp.`,
    remedy: `Planung einer Feuerwehr-Aufstellfläche nach DIN 14090 oder Ausführung eines zweiten baulichen Rettungswegs (Außentreppe).`
  },

  // 3. Kommunale Satzungen
  "muenchen_stellplatz": {
    id: "muenchen_stellplatz",
    norm: "§§ 2–5 StplS München",
    title: "Stellplatzsatzung der Landeshauptstadt München",
    jurisdiction: "Kommunale Satzung (München)",
    wortlaut: `§ 2 Verpflichtung zur Herstellung: Bei der Errichtung von Gebäuden mit Wohnungen ist je Wohnung mindestens ein Kraftfahrzeugstellplatz herzustellen (Schlüssel 1,0 Kfz/WE).
§ 5 Fahrradabstellplätze: Je Wohneinheit sind mindestens 2 leicht zugängliche, diebstahlsichere Fahrradabstellplätze nachzuweisen.`,
    commentary: `München hält strikt an der Realherstellung fest. Eine Stellplatzablöse ist nur in begründeten Ausnahmefällen in der Altstadt gegen erhebliche Ablösebeiträge möglich.`,
    remedy: `Tiefgaragenstellplatz oder oberirdischen Stellplatz nachweisen, alternativ formellen Antrag auf Stellplatzablöse bei der LBK München einreichen.`
  },
  "muenchen_gruendach": {
    id: "muenchen_gruendach",
    norm: "§ 4 Freiflächengestaltungssatzung (FGS München)",
    title: "Dachbegrünungspflicht München",
    jurisdiction: "Kommunale Satzung (München)",
    wortlaut: `§ 4 Abs. 1: Flachdächer und flach geneigte Dächer bis zu einer Neigung von 15 Grad bei baulichen Anlagen mit mehr als 100 m² überbauter Grundfläche sind dauerhaft und flächendeckend extensiv oder intensiv zu begrünen.
Abs. 2: Die durchwurzelbare Gesamtaufbaustärke der Begrünung muss bei extensiver Begrünung mindestens 10 cm betragen.`,
    commentary: `Dient dem Mikroklimaschutz, der Regenwasserrückhaltung und der Biodiversität im Stadtgebiet. Ausnahmen sind nur bei nachgewiesener technischer Unmöglichkeit (z. B. reine Glasdächer) statthaft.`,
    remedy: `Statische Auslegung der Decke um mind. 100–140 kg/m² Nasslast für das Gründachsubstrat ergänzen und im Baubeschrieb extensiv deklarieren.`
  },
  "berlin_stellplatz": {
    id: "berlin_stellplatz",
    norm: "§ 49 Abs. 1 BauO Bln",
    title: "Abschaffung der Kfz-Stellplatzpflicht in Berlin",
    jurisdiction: "Landesbauordnung Berlin",
    wortlaut: `§ 49 (1) Bauliche Anlagen, bei denen ein Zu- und Abgangsverkehr mit Kraftfahrzeugen zu erwarten ist, müssen Stellplätze für Kraftfahrzeuge nicht mehr nachweisen (Abschaffung der Kfz-Pflicht für Wohnungen).
(2) Es sind jedoch ausreichend Fahrradabstellplätze in geeigneter Weise auf dem Baugrundstück herzustellen (mindestens 2 je Wohneinheit).`,
    commentary: `Berlin hat als erstes Bundesland die Stellplatzpflicht für Wohnungen komplett abgeschafft, um das Bauen zu verbilligen und die Mobilitätswende zu fördern.`,
    remedy: `Kein Kfz-Nachweis nötig. Fokus voll auf adäquate, ebenerdige Fahrradstellplätze legen.`
  },

  // 4. Österreich (OIB & Wiener Bauordnung)
  "oib_rl_2": {
    id: "oib_rl_2",
    norm: "OIB-Richtlinie 2 (Brandschutz)",
    title: "Brandschutz & Gebäudeklassifizierung (Österreich)",
    jurisdiction: "OIB / Österreichisches Institut für Bautechnik",
    wortlaut: `Tabelle 1 Einstufung in Gebäudeklassen (GK 1 bis 5):
GK 1: Freistehende, nur von einer Seite zugängliche Wohngebäude mit nicht mehr als zwei oberirdischen Geschoßen und max. 400 m² BGF.
GK 2: Reihenhäuser und gekuppelte Wohngebäude bis zu zwei Geschoßen.
GK 3: Gebäude mit bis zu drei oberirdischen Geschoßen und nicht mehr als 600 m² BGF je Geschoß.
GK 4: Gebäude mit bis zu sechs oberirdischen Geschoßen und Fluchtniveau bis 22 m.
GK 5: Hochhäuser mit Fluchtniveau über 22 m.`,
    commentary: `Die OIB-Richtlinie 2 harmonisiert den baulichen Brandschutz in allen 9 Bundesländern Österreichs. Abweichend von der deutschen MBO wird hier stark mit Brutto-Grundflächenbegrenzungen (BGF) operiert.`,
    remedy: `Nachweis eines qualifizierten Brandschutzkonzepts gem. OIB-Leitfaden bei Überschreitung der BGF-Grenzwerte.`
  },
  "oib_rl_3": {
    id: "oib_rl_3",
    norm: "OIB-Richtlinie 3 (Hygiene & Gesundheit)",
    title: "Raumhöhen und Belichtung von Aufenthaltsräumen",
    jurisdiction: "OIB / Österreichisches Institut für Bautechnik",
    wortlaut: `Punkt 7 Raumhöhe: Aufenthaltsräume müssen eine lichte Raumhöhe von mindestens 2,50 m aufweisen. In Einfamilienhäusern sowie bei Dachschrägen kann die lichte Raumhöhe auf 2,40 m über mindestens der halben Bodenfläche reduziert werden.
Punkt 8 Belichtung: Aufenthaltsräume müssen direkt ins Freie über eine wirksame Belichtungsfläche von mindestens 10% der Bodenfläche des Raumes verfügen.`,
    commentary: `Strikte österreichweite Vorgabe für gesundes Wohnen. Ein Unterschreiten der 2,40 m Marke gilt baurechtlich als nicht bewilligungsfähiger Mangel.`,
    remedy: `Planung der Rohbauhöhe unter Berücksichtigung des Fußbodenaufbaus (Estrich + Trittschalldämmung ca. 14–18 cm) mind. auf 2,70 m ansetzen.`
  },
  "wien_bo_79": {
    id: "wien_bo_79",
    norm: "§ 79 BO für Wien",
    title: "Abstände der Gebäude von den Nachbargrenzen (Bauwich)",
    jurisdiction: "Bauordnung für Wien",
    wortlaut: `(1) Der Abstand der Gebäude von den Nachbargrenzen richtet sich nach der Gebäudehöhe, muss jedoch mindestens der halben Gebäudehöhe entsprechen, mindestens aber 3,0 m betragen (Mindestbauwich).
(2) In der geschlossenen Bauweise ist an die seitliche Grenze anzubauen.`,
    commentary: `Die Wiener Bauordnung unterscheidet scharf zwischen geschlossener, gekuppelter und offener Bauweise. Im Bauwich dürfen nur Nebengebäude bis max. 3 m Gebäudehöhe errichtet werden.`,
    remedy: `Prüfung der Bauklasse (I bis VI) im Flächenwidmungs- und Bebauungsplan der Stadt Wien.`
  },

  // 5. Schweiz (IVHB, PBG Zürich & SIA)
  "ivhb_grenzabstand": {
    id: "ivhb_grenzabstand",
    norm: "IVHB Art. 6 & Konkordat",
    title: "Grenzabstände nach Schweizer Baubegriffen",
    jurisdiction: "Interkantonale Vereinbarung (IVHB / Schweiz)",
    wortlaut: `Art. 6.1 Kleiner Grenzabstand: Der kleine Grenzabstand ist der kürzeste Abstand zwischen der projizierten Fassadenlinie und der Grundstücksgrenze für Fassaden ohne Hauptwohnfenster (mind. 3.50 m bis 5.00 m je nach kantonalem PBG).
Art. 6.2 Grosser Grenzabstand: Gilt gegenüber der am stärksten belichteten Wohnseite (häufig Süd- oder Westfassade).`,
    commentary: `Die IVHB harmonisiert die Definitionen; die konkreten Metermaße werden von den Kantonen und kommunalen Bau- und Zonenordnungen (BZO) festgelegt.`,
    remedy: `Grenzbaurechtliche Vereinbarung (Näherbaurecht) mit dem Nachbarn im Grundbuch eintragen lassen, falls der Mindestabstand unterschritten wird.`
  },
  "pbg_zh_293": {
    id: "pbg_zh_293",
    norm: "§ 293 PBG ZH",
    title: "Wohn- und Schlafräume, Lichte Raumhöhe (Kanton Zürich)",
    jurisdiction: "Planungs- und Baugesetz Kanton Zürich",
    wortlaut: `§ 293 (1) Wohn- und Schlafräume müssen eine lichte Höhe von mindestens 2,40 m aufweisen; bei Dachräumen genügt diese Höhe über mindestens der halben Bodenfläche.
(2) Ausnahmen können bewilligt werden bei Umbauten und historischen Bauten.`,
    commentary: `Im Kanton Zürich gilt der Mindestwert von 2.40 m. In Büro- und Gewerbebauten fordern die Arbeitsplatzrichtlinien der SUVA meist höhere Werte.`,
    remedy: `Bei Dachgeschossausbauten Kniestockhöhe und Neigung so austarieren, dass die 2.40 m Zone mindestens 50% der anrechenbaren Fläche umfasst.`
  },
  "sia_500": {
    id: "sia_500",
    norm: "SIA 500 / IVHB",
    title: "Hindernisfreie Bauten (Schweizer Norm)",
    jurisdiction: "Schweizerischer Ingenieur- und Architektenverein (SIA)",
    wortlaut: `Kapitel 9 Wohnungsbau: Wohnungszugänge müssen stufenlos erreichbar sein. Türen und Durchgänge müssen eine lichte Durchgangsbreite von mindestens 80 cm aufweisen. Korridore mind. 1.20 m Breite.`,
    commentary: `In der Schweiz Standard für alle Mehrfamilienhäuser ab 4 Wohneinheiten gemäss den kantonalen Baugesetzen.`,
    remedy: `Schwellenlose Übergänge auf Balkone und Terrassen mittels Rinnen und Hebe-Schiebetüren mit Niveauschwelle ausführen.`
  },

  // 6. Erweiterte Fachplaner-Normen (Brandschutz, Barrierefreiheit, Schallschutz)
  "mbo_30": {
    id: "mbo_30",
    norm: "§ 30 MBO",
    title: "Brandwände und Gebäudeabschlusswände",
    jurisdiction: "Musterbauordnung / Landesbauordnungen",
    wortlaut: `(2) Brandwände sind erforderlich als Gebäudeabschlusswand, wenn diese Gebäude weniger als 2,50 m von der Grundstücksgrenze errichtet werden, es sei denn, dass ein Abstand von mindestens 5 m zu bestehenden oder nach den baurechtlichen Vorschriften zulässigen künftigen Gebäuden öffentlich-rechtlich gesichert ist.
(3) Brandwände müssen als raumabschließende Bauteile aus nichtbrennbaren Baustoffen (F90-A bzw. REI 90-M) so beschaffen sein, dass sie unter zusätzlicher mechanischer Beanspruchung ihre Tragfähigkeit und den Raumabschluss nicht verlieren.`,
    commentary: `Schutz vor Brandüberschlag auf das Nachbargrundstück. Fenster oder brennbare Einbauten sind in Brandwänden unzulässig.`,
    remedy: `Ausführung der grenzständigen Wand in massiver Stahlbeton- oder Kalksandsteinbauweise (REI 90-M) ohne Öffnungen oder Eintragung einer Baulast/Dienstbarkeit zur Abstandsflächenübernahme.`
  },
  "mbo_50": {
    id: "mbo_50",
    norm: "§ 50 MBO / DIN 18040-2",
    title: "Barrierefreie Wohnungen und Aufzugsanlagen",
    jurisdiction: "Musterbauordnung / Landesbauordnungen",
    wortlaut: `(1) In Gebäuden mit mehr als zwei Wohnungen müssen die Wohnungen eines Geschosses barrierefrei erreichbar sein; diese Wohnungen müssen barrierefrei nutzbar sein.
(2) Gebäude mit einer Höhe nach § 2 Abs. 3 Satz 2 von mehr als 13 m oder mit mehr als drei Vollgeschossen müssen Aufzüge in ausreichender Zahl haben. Mindestens ein Aufzug muss für Rollstühle, Krankentragen und Lasten geeignet sein (Mindestkabinenmaß 1,10 m x 2,10 m).`,
    commentary: `Zwingende Pflicht ab Geschossschwelle (je nach Bundesland 3 bis 4 Vollgeschosse). Fehlender Aufzug führt zur Zurückweisung des Bauantrags.`,
    remedy: `Planung eines standardisierten Durchlader-Seil- oder Hydraulikaufzugs im Treppenhauskern.`
  },
  "ta_laerm": {
    id: "ta_laerm",
    norm: "TA Lärm / DIN 4109",
    title: "Schallschutz gegen Außenlärm & Nachbarschutz Haustechnik",
    jurisdiction: "Bundes-Immissionsschutzgesetz (BImSchG) / DIN 4109",
    wortlaut: `Punkt 6.1 Immissionsrichtwerte: In reinen Wohngebieten tags 50 dB(A), nachts 35 dB(A). In allgemeinen Wohngebieten tags 55 dB(A), nachts 40 dB(A).
DIN 4109 Schallschutz im Hochbau: Bauteile müssen Geräusche von haustechnischen Anlagen so dämmen, dass der zulässige Innenraumpegel von 30 dB(A) nachts nicht überschritten wird.`,
    commentary: `Besonders Außenaufstellungen von Wärmepumpen, Klimasplitgeräten und Lüftungszentralen müssen den Nachbarimmissionsort (maßgebliches Fenster des Nachbarn) berücksichtigen.`,
    remedy: `Mindestabstand von 3 m zur Grenze wahren, Ausblasrichtung vom Nachbarn abwenden oder akkreditierte Schallschutzhaube mit mind. 6–10 dB(A) Schalldämmung einplanen.`
  },

  // 7. Bebauungsplan & Bauplanungsrecht (BauGB / BauNVO)
  "baunvo_23": {
    id: "baunvo_23",
    norm: "§ 23 BauNVO",
    title: "Überbaubare Grundstücksfläche, Baugrenze & Baulinie",
    jurisdiction: "Baunutzungsverordnung (BauNVO)",
    wortlaut: `(2) Ist eine Baulinie festgesetzt, so muß auf dieser Linie gebaut werden. Ein Vor- oder Zurücktreten von Gebäudeteilen in geringfügigem Ausmaß kann zugelassen werden.
(3) Ist eine Baugrenze festgesetzt, so dürfen Gebäude und Gebäudeteile diese nicht überschreiten. Ein Vortreten von Gebäudeteilen in geringfügigem Ausmaß kann zugelassen werden.`,
    commentary: `Baugrenzen umgrenzen das überbaubare Baufenster. Überschreitungen sind formelle Baurechtsverstöße, die zwingend einen Befreiungsantrag nach § 31 Abs. 2 BauGB erfordern.`,
    remedy: `Anpassung des Gebäudegrundrisses an das Baufenster oder frühzeitige Beantragung einer städtebaulichen Befreiung bei der Gemeinde.`
  },
  "baugb_30": {
    id: "baugb_30",
    norm: "§ 30 BauGB",
    title: "Zulässigkeit von Vorhaben im Geltungsbereich eines Bebauungsplans",
    jurisdiction: "Baugesetzbuch (BauGB)",
    wortlaut: `(1) Im Geltungsbereich eines qualifizierten Bebauungsplans ist ein Vorhaben zulässig, wenn es den Festsetzungen des Bebauungsplans nicht widerspricht und die Erschließung gesichert ist.`,
    commentary: `Qualifizierte B-Pläne regeln mindestens Art und Maß der baulichen Nutzung, überbaubare Grundstücksflächen und örtliche Verkehrsflächen. Widersprüche führen unmittelbar zur Ablehnung des Baugesuchs.`,
    remedy: `Prüfung der Festsetzungen über Bauweise, Dachform, First- und Traufhöhe vor Genehmigungsplanung.`
  }
};

/**
 * Searches the statutes database for the best matching legal paragraph.
 *
 * @param {string} query - Statute query or rule id (e.g. "§6 BayBO", "abstandsflaeche", "grz")
 * @param {string} [landCode="MBO"] - Context Bundesland code
 * @param {string} [muniCode=null] - Context municipality code
 * @returns {object|null} Matched statute object
 */
export function getStatuteByQuery(query, landCode = "MBO", muniCode = null) {
  if (!query || typeof query !== "string") return null;
  const q = query.toLowerCase().trim();

  // Direct ID check
  if (STATUTES_DB[q]) return STATUTES_DB[q];

  const lUpper = (landCode || "").toUpperCase();

  // Check Austria (OIB / Wien)
  if (lUpper.startsWith("AT") || q.includes("oib") || q.includes("österreich") || q.includes("oesterreich")) {
    if (q.includes("brand") || q.includes("klasse") || q.includes("gebaeudeklasse") || q.includes("gk") || q.includes("rl 2")) {
      return STATUTES_DB["oib_rl_2"];
    }
    if (q.includes("höhe") || q.includes("hoehe") || q.includes("raum") || q.includes("belichtung") || q.includes("tageslicht") || q.includes("rl 3")) {
      return STATUTES_DB["oib_rl_3"];
    }
    if (lUpper === "AT_W" || q.includes("wien") || q.includes("bauwich") || q.includes("79")) {
      return STATUTES_DB["wien_bo_79"];
    }
  }

  // Check Switzerland (IVHB / Zürich / SIA)
  if (lUpper.startsWith("CH") || q.includes("ivhb") || q.includes("schweiz") || q.includes("suisse") || q.includes("pbg")) {
    if (q.includes("abstand") || q.includes("grenz") || q.includes("näherbau") || q.includes("art. 6")) {
      return STATUTES_DB["ivhb_grenzabstand"];
    }
    if (lUpper === "CH_ZH" || q.includes("zürich") || q.includes("zuerich") || q.includes("293")) {
      return STATUTES_DB["pbg_zh_293"];
    }
    if (q.includes("barriere") || q.includes("hindernis") || q.includes("sia") || q.includes("500")) {
      return STATUTES_DB["sia_500"];
    }
  }

  // Check municipal rules
  if (muniCode === "MUENCHEN" || q.includes("münchen") || q.includes("muenchen") || q.includes("fgs")) {
    if (q.includes("dach") || q.includes("grün") || q.includes("fgs") || q.includes("freiflächen")) {
      return STATUTES_DB["muenchen_gruendach"];
    }
    if (q.includes("stellplatz") || q.includes("pkw") || q.includes("stpls")) {
      return STATUTES_DB["muenchen_stellplatz"];
    }
    return STATUTES_DB["muenchen_gruendach"];
  }
  if ((landCode === "BE" || muniCode === "BERLIN" || q.includes("berlin") || q.includes("bln")) && (q.includes("stellplatz") || q.includes("49"))) {
    return STATUTES_DB["berlin_stellplatz"];
  }

  // Check state-specific setback
  if (q.includes("abstand") || q.includes("grenz") || q.includes("§6") || q.includes("§5") || q.includes("§22")) {
    if (landCode === "BW" || q.includes("bw") || q.includes("baden")) return STATUTES_DB["lbo_bw_6"];
    if (landCode === "BY" || q.includes("by") || q.includes("bayern") || q.includes("baybo")) return STATUTES_DB["baybo_22"];
    if (landCode === "NW" || q.includes("nrw") || q.includes("nordrhein")) return STATUTES_DB["bauo_nrw_6"];
    if (landCode === "HE" || q.includes("hessen") || q.includes("hbo")) return STATUTES_DB["hbo_6"];
    if (landCode === "NI" || q.includes("niedersachsen") || q.includes("nbauo")) return STATUTES_DB["nbauo_5"];
    if (landCode === "HH" || q.includes("hamburg") || q.includes("hbauo")) return STATUTES_DB["hbauo_6"];
    return STATUTES_DB["mbo_6"];
  }

  // Check GRZ / GFZ
  if (q.includes("grz") || q.includes("19")) return STATUTES_DB["baunvo_19"];
  if (q.includes("gfz") || q.includes("20")) return STATUTES_DB["baunvo_20"];
  if (q.includes("vollgeschoss") || q.includes("16")) return STATUTES_DB["baunvo_16"];

  // Check Building class & Safety
  if (q.includes("brandwand") || q.includes("gebäudeabschluss") || q.includes("30")) return STATUTES_DB["mbo_30"];
  if (q.includes("aufzug") || q.includes("barriere") || q.includes("rollstuhl") || q.includes("50")) return STATUTES_DB["mbo_50"];
  if (q.includes("schall") || q.includes("lärm") || q.includes("laerm") || q.includes("wärmepumpe") || q.includes("waermepumpe") || q.includes("4109")) return STATUTES_DB["ta_laerm"];
  if (q.includes("gebaeudeklasse") || q.includes("klasse") || q.includes("gk") || q.includes("§2")) return STATUTES_DB["mbo_2"];
  if (q.includes("rettungsweg") || q.includes("treppenraum") || q.includes("brand") || q.includes("14") || q.includes("33")) return STATUTES_DB["mbo_14"];
  if (q.includes("tageslicht") || q.includes("licht") || q.includes("fenster") || q.includes("47") || q.includes("din 5034")) return STATUTES_DB["mbo_47"];
  if (q.includes("raumhoehe") || q.includes("höhe") || q.includes("46")) return STATUTES_DB["mbo_46"];

  // Check B-Plan & Zoning rules
  if (q.includes("bplan") || q.includes("baugrenze") || q.includes("baulinie") || q.includes("23")) return STATUTES_DB["baunvo_23"];
  if (q.includes("baugb") || q.includes("bebauungsplan") || q.includes("30")) return STATUTES_DB["baugb_30"];

  // Fallback to MBO 6
  return STATUTES_DB["mbo_6"];
}
