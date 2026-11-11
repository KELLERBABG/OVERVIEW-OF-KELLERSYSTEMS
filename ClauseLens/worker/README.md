# ClauseLens Lizenzserver (Cloudflare Worker)

Prüft signierte `CL-` Schlüssel, validiert Polar-Schlüssel (`CLA-`, `LENS-`) online und stellt gerätegebundene Sitzungstoken (RS256, 7 Tage) aus. Er ist der einzige Aussteller von `CL-` Schlüsseln.

## Endpunkte

- `GET /api/v1/health`
- `POST /api/v1/seats/verify` mit `{ licenseKey, machineId, email? }` – `email` ist optional, `licenseKey` und `machineId` sind Pflicht. Antwortet mit einem signierten Sitzungstoken (`token`), das CLI und Browser lokal prüfen.
- `POST /api/v1/webhook/polar` (Polar `order.created`, Standard-Webhooks-Signatur). Stellt einen `CL-` Schlüssel aus und mailt ihn per Resend an `order.customer.email`.

## Deploy

```bash
npx wrangler deploy --config worker/wrangler.toml
npx wrangler secret put POLAR_WEBHOOK_SECRET
npx wrangler secret put RESEND_API_KEY       # Mailversand der Lizenzschlüssel
npx wrangler secret put LICENSE_SECRET       # HMAC-Secret für CL- Schlüssel (nur hier, nicht im Paket)
npx wrangler secret put LICENSE_SIGNING_KEY  # RSA-2048 PKCS8 (base64), signiert die Sitzungstoken
```

Secrets werden beim Lesen getrimmt: ein Zeilenumbruch, den `wrangler secret put` beim Durchpipen anhängt, ist damit unschädlich. Eine Rotation von `LICENSE_SECRET` macht bereits ausgestellte `CL-` Schlüssel ungültig (`CLA-`/`LENS-` Schlüssel von Polar bleiben gültig, weil sie online bei Polar geprüft werden).

Ohne `LICENSE_SIGNING_KEY` antwortet `/api/v1/seats/verify` mit 503, ohne `LICENSE_SECRET` verweigert der Webhook die Ausstellung. Der öffentliche Schlüssel zu `LICENSE_SIGNING_KEY` steht als `LICENSE_TOKEN_PUBLIC_KEY` in `src/licence-token.js` (re-exportiert aus `src/licensing.js`); beide müssen zusammengehören, sonst lehnen CLI und Browser die Token ab.

`wrangler.toml` bindet den Worker an die Custom Domain `license.kellersystems.dev`; die Zone muss im selben Cloudflare-Account liegen. `workers_dev = true` hält zusätzlich `https://clauselens-license.papababg02.workers.dev` aktiv: Bot Fight Mode der Zone beantwortet Aufrufe ohne Browser auf der Custom Domain mit HTTP 403, CLI und Browser weichen dann auf diese Adresse aus. Im Polar-Dashboard den Webhook auf `https://license.kellersystems.dev/api/v1/webhook/polar` zeigen lassen.

Absender ist `ClauseLens <lizenz@kellersystems.dev>`, änderbar über `LICENSE_FROM_EMAIL`. Die Absenderdomain muss bei Resend verifiziert sein, sonst antwortet Resend mit HTTP 403 ("domain not verified"), der Worker gibt 502 zurück und Polar wiederholt die Zustellung. Ohne `RESEND_API_KEY` wird der Schlüssel weiter ausgestellt, aber nicht gemailt (`emailSent: false` im Delivery-Log).

## Lokal prüfen

```bash
node --test test/worker.test.js
npx wrangler dev worker/index.js --config worker/wrangler.toml
```
## Deploy aus dem DSH-Desktop (Windows)

Im DSH-Desktop läuft `node` als eingebettetes Electron (`process.versions.electron` gesetzt). Wranglers Argument-Parser erkennt dann die CLI-Argumente nicht (`Unknown arguments: <cli.js>, ...`, `yargs validation error`). Ein Preload neutralisiert die Electron-Marker für den Aufruf:

```js
// .secrets/wrangler-preload.cjs
try { delete process.versions.electron; } catch {}
try { delete process.versions.chrome; } catch {}
```

```powershell
npm.cmd install --no-save --no-package-lock wrangler@4
node --require ./.secrets/wrangler-preload.cjs node_modules\wrangler\bin\wrangler.js deploy --config worker/wrangler.toml
```

Der Preload wird an Wranglers Kindprozesse vererbt. Secrets genauso setzen, z. B. `$value | node --require ./.secrets/wrangler-preload.cjs node_modules\wrangler\bin\wrangler.js secret put NAME --config worker/wrangler.toml` (der Zeilenumbruch aus der Pipe wird beim Lesen getrimmt). `node_modules/` ist gitignored, `.secrets/` ebenfalls.
