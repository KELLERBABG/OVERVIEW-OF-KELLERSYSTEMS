# Akquise-Mails

Ein kleiner Mailer für Kalt- und Bestandskundenansprache. Kein SaaS, keine zusätzliche Abhängigkeit:
Versand über Resend im Free-Tarif oder über ein eigenes SMTP-Postfach, Empfängerliste als CSV, Vorlage als
Textdatei. Das Tool prüft vor jedem Versand, ob für den Empfänger eine Rechtsgrundlage dokumentiert ist,
und verweigert sonst den Versand.

## Rechtlicher Rahmen

Werbe-Mail an Unternehmen braucht in Deutschland eine Rechtsgrundlage nach § 7 UWG:

- `einwilligung`: dokumentierte Einwilligung des Empfängers (Double-Opt-In, Unterschrift, E-Mail mit Bestätigung). Pflichtfeld `einwilligung_am` im Format JJJJ-MM-TT.
- `bestandskunde`: bestehende Kundenbeziehung nach § 7 Abs. 3 UWG. Voraussetzung: Adresse im Zusammenhang mit einem Kauf oder Vertrag erhalten, Werbung für eigene ähnliche Leistungen, klare Abmeldemöglichkeit.

Für jede Zeile ist zusätzlich `quelle` Pflicht (z.B. "Double-Opt-In 2026-10-01" oder "Bestellung 2026-09-15,
Kundenkonto 42"). Rollenadressen wie `noreply@`, `postmaster@` oder `abuse@` werden übersprungen.
Ohne gültige Rechtsgrundlage wird die Zeile nicht versendet, auch nicht mit `--send`.

Jede Mail enthält automatisch einen Footer mit Absender, Anschrift, Abmeldung und Widerspruchsadresse sowie
den Header `List-Unsubscribe`. Abmeldungen müssen manuell in die Sperrliste (siehe unten), das Tool liest
kein Postfach.

## Installation und Konfiguration

Keine Installation nötig. Variablen in `.env` im Projektwurzelverzeichnis (wird von `npm run outreach`
automatisch gelesen) oder in der Umgebung:

```bash
# Variante A: Resend (Free: 3.000 Mails/Monat, 100 pro Tag)
RESEND_API_KEY=re_...

# Variante B: eigenes SMTP-Postfach
SMTP_HOST=mail.example.de
SMTP_PORT=587
SMTP_SECURE=false
SMTP_USER=info@example.de
SMTP_PASS=...
SMTP_REQUIRE_STARTTLS=true

# Absenderdaten (Defaults in Klammern)
OUTREACH_FROM_NAME=Lukas Negenborn
OUTREACH_FROM_EMAIL=info@kellersystems.dev
OUTREACH_COMPANY=Keller Systems Software Development
OUTREACH_ADDRESS=c/o Block Services, Stuttgarter Str. 106, 70736 Fellbach
OUTREACH_SITE=https://clause.kellersystems.dev
OUTREACH_UNSUBSCRIBE=info@kellersystems.dev
```

Bei Resend die Absenderdomain verifizieren (SPF und DKIM eintragen). Ohne verifizierte Domain versendet
Resend nur an die eigene Adresse.

## Empfängerliste

CSV mit Semikolon oder Komma, Kopfzeile Pflicht. Beispiel: `tools/outreach/example-contacts.csv`.

```csv
email;name;firma;rechtsgrundlage;einwilligung_am;quelle;subject
max@example.de;Max Mustermann;Muster GmbH;einwilligung;2026-10-01;Double-Opt-In 01.10.2026;Kurze Frage zu Ihren AGB
```

- `subject` ist optional (Default: "Kurze Frage zu Ihren AGB"), Platzhalter sind dort erlaubt.
- Weitere Spalten sind frei nutzbar und in der Vorlage als `{{spaltenname}}` verfügbar.

## Vorlage

Textdatei mit Platzhaltern, Beispiel: `tools/outreach/example-template.txt`.

Verfügbar: `{{vorname}}`, `{{nachname}}`, `{{name}}`, `{{anrede}}`, `{{firma}}`, `{{absender}}`,
`{{firma_absender}}`, `{{website}}`, `{{abmelde_adresse}}` plus alle CSV-Spalten. Unbekannte Platzhalter
bleiben stehen, damit Tippfehler sichtbar werden. Der Footer wird automatisch angehängt.

## Befehle

```bash
# Vorschau der ersten drei Mails, ohne Versand
npm run outreach -- preview --list tools/outreach/example-contacts.csv --template tools/outreach/example-template.txt

# Dry-Run: zeigt, was passieren würde, sendet nichts
npm run outreach -- send --list kontakte.csv --template vorlage.txt

# Echter Versand, 20 Sekunden Pause zwischen Mails
npm run outreach -- send --list kontakte.csv --template vorlage.txt --send --delay 20

# Nur 10 Mails in diesem Lauf
npm run outreach -- send --list kontakte.csv --template vorlage.txt --send --limit 10

# Adresse sperren
npm run outreach -- suppress --email antwort@example.de

# Liste gesperrter Adressen
npm run outreach -- suppress --file abmeldungen.txt
```

Alternativ direkt: `node tools/outreach.mjs ...`.

## Zustand und Idempotenz

- `.outreach/state.jsonl` protokolliert jeden Versand (`sent` oder `failed`) mit Zeitstempel. Bereits
  erfolgreich angeschriebene Adressen werden bei weiteren Läufen übersprungen.
- `.outreach/suppressed.txt` enthält gesperrte Adressen, eine pro Zeile. Beide Dateien sind über
  `.gitignore` ausgeschlossen und dürfen nicht ins Repository.
- Duplikate in der Liste und Adressen ohne Rechtsgrundlage landen unter "Übersprungen" mit Begründung.

## Deliverability

- Eigene Subdomain für den Versand (z.B. `mail.example.de`) statt der Hauptdomain, SPF, DKIM und DMARC setzen.
- Kleine Mengen, Pausen zwischen Mails (`--delay`), keine Anhänge, keine Bilder, ein klarer Betreff.
- Bounces und Beschwerden sofort in `suppress` aufnehmen. Bei Resend den Zustellstatus im Dashboard prüfen.
- 100 Mails pro Tag im Resend-Free-Tarif. Für größere Mengen kostenpflichtiger Tarif oder eigenes SMTP.

## Checkliste vor dem ersten Versand

1. Rechtsgrundlage für jede Zeile dokumentiert (`rechtsgrundlage`, `quelle`, ggf. `einwilligung_am`).
2. Absenderdomain verifiziert (Resend) oder SMTP-Zugang getestet (`send` ohne `--send` ist nur Dry-Run).
3. Testmail an die eigene Adresse senden und Darstellung prüfen.
4. Abmeldungen und Bounces in `suppress` eintragen.
5. Impressum und Datenschutzerklärung der beworbenen Website prüfen.
