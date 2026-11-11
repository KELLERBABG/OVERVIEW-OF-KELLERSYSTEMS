"use strict";

/**
 * ClauseLens Archil-Speicher — serverseitige Persistenz fuer Dateien, die einen
 * Lauf ueberleben sollen.
 *
 * Der ClauseLens-Kern bleibt offline: dieses Modul ist die einzige Stelle, die
 * einen externen Dienst anspricht, und wird nirgends automatisch geladen. Der
 * API-Key liegt ausschliesslich serverseitig in `ARCHIL_API_KEY` — nie hinter
 * einem browser-sichtbaren Praefix (NEXT_PUBLIC_/VITE_) und nie in Client-Code.
 *
 * Ein Archil-Disk ist ein persistierendes Dateisystem: jeder `exec` laeuft in
 * einem frischen Container, der Disk-Inhalt bleibt. Damit eignen sich Disks fuer
 * generierte Vertragskorpora, Audit-Exporte und andere Artefakte, die ein
 * einzelner Prozesslauf nicht ueberdauern wuerde.
 *
 * Konfiguration (siehe .env.example):
 *   ARCHIL_API_KEY  API-Key vom Typ "key-…" (Konto-Ebene, region-gebunden)
 *   ARCHIL_REGION   Region des Keys, Standard aws-us-east-1
 *   ARCHIL_DISK_ID  ID des verwendeten Disks ("dsk-…")
 */

import * as archil from "disk";

export const DEFAULT_ARCHIL_REGION = "aws-us-east-1";

/** Aktive Region: `ARCHIL_REGION` oder der Standard. */
export function archilRegion() {
  return process.env.ARCHIL_REGION || DEFAULT_ARCHIL_REGION;
}

/** ID des verwendeten Disks aus der Umgebung. */
export function archilDiskId() {
  const id = process.env.ARCHIL_DISK_ID;
  if (!id) {
    throw new Error(
      "ARCHIL_DISK_ID ist nicht gesetzt. Zuerst ein Disk anlegen (z.B. via " +
        "`node bin/archil-persist-check.js`) und seine ID (\"dsk-…\") als " +
        "ARCHIL_DISK_ID bereitstellen."
    );
  }
  return id;
}

/**
 * Konfiguriert den SDK-Client aus der Umgebung. Faellt auf die von `disk` selbst
 * gelesenen Variablen ARCHIL_API_KEY / ARCHIL_REGION zurueck.
 *
 * @returns {{ region: string }} die aktive Region
 */
export function configureArchil({
  apiKey = process.env.ARCHIL_API_KEY,
  region = archilRegion(),
} = {}) {
  if (!apiKey) {
    throw new Error(
      'ARCHIL_API_KEY ist nicht gesetzt. Einen API-Key (beginnt mit "key-", ' +
        "kein Disk-/Mount-Token) unter https://console.archil.com/api-keys " +
        "anlegen und serverseitig als ARCHIL_API_KEY bereitstellen."
    );
  }
  archil.configure({ apiKey, region });
  return { region };
}

/**
 * Liefert den in `ARCHIL_DISK_ID` benannten Disk.
 *
 * @param {string} [id] Disk-ID; ohne Angabe ARCHIL_DISK_ID
 */
export async function getDisk(id = archilDiskId()) {
  configureArchil();
  return archil.getDisk(id);
}

/**
 * Fuehrt einen Shell-Befehl auf dem Disk aus und liefert
 * `{ stdout, stderr, exitCode, timing }`. Jeder Aufruf ist ein frischer
 * Container mit dem Disk als Arbeitsverzeichnis; Dateien auf dem Disk bleiben
 * erhalten, Container-Zustand nicht.
 */
export async function execOnDisk(command, id = archilDiskId()) {
  const disk = await getDisk(id);
  return disk.exec(command);
}

/** Schreibt einen String als Objekt auf den Disk (S3-kompatibler Schluessel). */
export async function writeDiskFile(key, body, id = archilDiskId()) {
  const disk = await getDisk(id);
  return disk.putObject(key, body);
}

/** Liest ein Objekt vom Disk und liefert es als UTF-8-String. */
export async function readDiskFile(key, id = archilDiskId()) {
  const disk = await getDisk(id);
  const bytes = await disk.getObject(key);
  return new TextDecoder("utf-8").decode(bytes);
}

/**
 * Liefert den Disk aus `ARCHIL_DISK_ID` oder legt einen neuen an. Der einmalige
 * Mount-Token des neuen Disks wird bewusst nie ausgegeben.
 *
 * @returns {Promise<{ disk: object, created: boolean }>}
 */
export async function resolveDisk({ id = process.env.ARCHIL_DISK_ID, name = "clauselens" } = {}) {
  configureArchil();
  if (id) {
    return { disk: await archil.getDisk(id), created: false };
  }
  const { disk } = await archil.createDisk({ name });
  return { disk, created: true };
}
