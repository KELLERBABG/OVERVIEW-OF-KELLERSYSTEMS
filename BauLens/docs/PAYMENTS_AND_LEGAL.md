# Zahlungen, Lizenzen und Rechtstexte

Diese Datei beschreibt, wie Preise, Checkout und Rechtstexte in diesem Repository zusammenhängen
und was bei einer Änderung zu tun ist.

## Kurzfassung

- **`site.config.json` ist die einzige Quelle** für Preise, Checkout-Links, Anbieterangaben und die
  Download-Matrix des Hubs.
- **`scripts/build-site.mjs` erzeugt daraus** die Rechtsseiten, die Preisseite, die
  Kaufbestätigung und den Preisblock der Startseite.
- **`scripts/check-links.mjs` prüft** anschließend alle internen Verweise.
- **Zahlungen laufen über Polar** als Merchant of Record. Es gibt kein Backend und keinen
  geheimen Schlüssel im Repository; der Kauf ist ein normaler Checkout-Link.

```powershell
node scripts/build-site.mjs     # Seiten neu erzeugen
node scripts/check-links.mjs    # Verweise prüfen
node scripts/check-prices.mjs   # Anzeige gegen site.config.json prüfen
node scripts/check-versions.mjs # Versionsstände prüfen
```

## Preise und Checkout

| Tarif | Preis | Laufzeit | Polar-Produkt |
|---|---|---|---|
| Starter (Projekt-Pass) | 49 € einmalig | ein Vorhaben, 30 Tage | `checkoutUrl: null` — noch anzulegen |
| Pro Architekt | 89 € / Monat (890 € / Jahr) | monatlich kündbar, 1 Arbeitsplatz | `checkoutUrl: null` — noch anzulegen |
| Büro / Kanzlei | 199 € / Monat (1.990 € / Jahr) | monatlich kündbar, 3 Arbeitsplätze inklusive | `checkoutUrl: null` — noch anzulegen |
| Enterprise / Serien | ab 1.490 € / Monat | nach Vereinbarung | auf Anfrage per E-Mail |

Die Preise werden in Euro geführt (`currency: EUR`) und erscheinen auf allen Oberflächen in dieser
Währung. Die Steuer ist im Preis enthalten (`tax_behavior: inclusive`); die Berechnung erfolgt im
Checkout anhand der Rechnungsadresse.

> **Offener Punkt in diesem Repository.** Mit dem Preismodellwechsel sind die alten
> Polar-Checkout-Links entfernt: sie zeigten auf Produkte in US-Dollar und hätten einen anderen
> Betrag abgebucht, als die Seite anzeigt. Solange die Produkte nicht in Polar angelegt sind,
> führt jeder Kauf-Button in den Download-Hub, und die Desktop-Anwendung läuft 14 Tage ohne
> Schlüssel. Sobald ein Produkt existiert, wird sein Checkout-Link in `tiers[].checkoutUrl`
> eingetragen; die Seiten zeigen ihn automatisch und `scripts/check-prices.mjs` prüft ihn mit.

**Einen Preis ändern:** `tiers[].price` (und `annualPrice`, falls vorhanden) in `site.config.json`
anpassen, dann `node scripts/build-site.mjs` ausführen. Formatiert wird in `scripts/pricing-format.mjs`,
weil die Anzeige und die Prüfung dieselbe Zeichenkette benutzen müssen. Achte darauf, dass der Wert
zum Produkt in Polar passt — die Anzeige folgt der Konfiguration, nicht dem Shop.

**Ein Produkt in Polar anlegen:** Im Polar-Dashboard ein Produkt mit dem gewünschten Preis und als
Benefit einen **License Key** hinzufügen. Polar zeigt dann einen Checkout-Link der Form
`https://buy.polar.sh/polar_cl_…` an, den du in `site.config.json` unter `checkoutUrl` einträgst.

## Anbieterangaben

Alle Rechtsseiten lesen die Anbieterangaben aus `legal` in `site.config.json`. Eine Änderung an
Adresse, E-Mail oder Umsatzsteuerhinweis wirkt sich damit gleichzeitig auf Impressum,
Datenschutzerklärung, AGB und Widerrufsbelehrung aus.

## Erzeugte Seiten

| Pfad | Inhalt | Quelle |
|---|---|---|
| `legal/impressum/index.html` | Anbieterkennzeichnung nach § 5 DDG | `legal` |
| `legal/datenschutz/index.html` | Datenschutzerklärung nach Art. 13/14 DSGVO | `legal`, `billing`, `hosting` |
| `legal/agb/index.html` | Allgemeine Geschäftsbedingungen | `billing`, `legal` |
| `legal/lizenz/index.html` | Endnutzer-Lizenzvereinbarung | `product` |
| `legal/widerruf/index.html` | Widerrufsbelehrung inkl. § 356 Abs. 5 BGB | `legal` |
| `pricing/index.html` | Preisübersicht, Leistungsvergleich, Checkout | `tiers`, `enterprise` |
| `checkout/success/index.html` | Kaufbestätigung und nächste Schritte | `billing` |
| `downloads/index.html` | Download-Hub mit OS-Erkennung und Direktverweisen | `downloads`, `product`, `billing` |
| `index.html` | Abschnitt zwischen `<!-- PRICING:START -->` und `<!-- PRICING:END -->` | `tiers` |

Der Preisblock der Startseite wird zwischen den beiden Markern ersetzt. **Ändere ihn nicht von
Hand** — beim nächsten Build wird er überschrieben.

## Testphase und Lizenz in der Desktop-Anwendung

Die Desktop-Anwendung läuft **14 Tage vollständig frei** („Projekt-Pass“-Logik: ab dem ersten
Start, kein Account). Danach verlangt ein Bildschirmschirm einen Polar-Lizenzschlüssel; ohne
brauchbaren Schlüssel bleibt die Arbeitsfläche gesperrt. Projekte und Plandaten bleiben davon
unberührt erhalten. Die Prüf-Engine selbst bleibt uneingeschränkt — die Kommandozeile und die
Regeln kennen keine Lizenz, nur die Desktop-Anwendung wird gesperrt.

Die Aktivierung prüft den Schlüssel gegen

```
POST https://api.polar.sh/v1/customer-portal/license-keys/validate
{ "key": "…", "organization_id": "6898f4c5-7bc8-4ec5-bc97-a16b69b0498c" }
```

Dieser Endpunkt ist laut Polar-Dokumentation **bewusst ohne Authentifizierung** nutzbar und für
öffentliche Clients wie Desktop- und Web-Anwendungen gedacht. Deshalb steht hier kein geheimer
Schlüssel, und es muss kein Backend betrieben werden. Gesendet wird ausschließlich der Schlüssel;
Plandaten verlassen den Rechner nie.

Geprüft und bestätigt (siehe `test/licensing.test.js`):

- Ein ungültiger Schlüssel liefert `404` mit `{"error":"ResourceNotFound"}`; die Anwendung zeigt
  daraufhin „unbekannt, widerrufen oder abgelaufen“ und lässt die Eingabe offen.
- Ein `200` mit Status `revoked` oder `expired` wird als Ablehnung gewertet — der Client glaubt dem
  Statusfeld, nicht dem Statuscode.
- Ist Polar nicht erreichbar (Netzwerkfehler, 5xx), unterscheidet die Anwendung sauber zwischen
  „Schlüssel schlecht“ und „Prüfung nicht möglich“. Eine zuvor geprüfte Lizenz bleibt dann
  **30 Tage** gültig (Grace-Fenster, `OFFLINE_GRACE_DAYS` in `src/licensing.js`); danach ist eine
  erneute Prüfung fällig, bevor die Anwendung wieder entsperrt wird. Offline ist ein
  unterstützter Zustand, kein Fehlschlag.

Der Zustand (Startzeitpunkt der Testphase, letzte erfolgreiche Prüfung, Ablaufdatum der Lizenz)
liegt im **eigenen Konfigurationsverzeichnis der Anwendung** (`license-state.json`, geschrieben
über die Shell-Befehle `license_state_read` / `license_state_write`) — absichtlich außerhalb des
Webview-Speichers: eine bereinigte Browserprofil-Datenbank setzt die Testphase nicht zurück.
Die Entscheidungslogik liegt getrennt davon in `src/licensing.js` und ist Teil der Testsuite.

## Hosting und Rechtstexte

Die Datenschutzerklärung nennt **Cloudflare Pages** als Hoster. Das entspricht dem Zielzustand;
zum Zeitpunkt der Erstellung lieferte `lens.kellersystems.dev` allerdings noch **GitHub Pages**
aus (`Server: GitHub.com`). Beim Umzug auf Cloudflare ist die Datei `_redirects` bereits
vorbereitet und wird dort wirksam.

**Falls die Seite länger auf GitHub Pages bleibt**, muss Abschnitt 3 der Datenschutzerklärung auf
GitHub, Inc. umgestellt werden — sonst benennt sie den falschen Empfänger der Server-Logfiles.
Die Angaben stehen in `site.config.json` unter `legal.hosting`.

## Was hier nicht automatisiert ist

- **Steuerliche Würdigung.** Ob die Kleinunternehmerregelung nach § 19 UStG bei Verkäufen über
  einen Merchant of Record greift, ist eine steuerliche Frage und hier nicht bewertet.
- **AGB-Kontrolle.** Die AGB und die Lizenzvereinbarung sind sorgfältig formuliert, aber nicht
  anwaltlich geprüft.
- **Widerrufsrecht bei digitalen Inhalten.** Die Belehrung gibt den Stand des § 356 Abs. 5 BGB
  wieder. Ob Polar die erforderliche ausdrückliche Zustimmung im Checkout so einholt, dass das
  Widerrufsrecht wirksam erlischt, gehört in den Checkout-Test vor dem Verkaufsstart.
