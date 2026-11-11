import { LetterMetadata } from "./extractor.js";

export class ReplyGenerator {
  public static generateStandardFristverlaengerung(meta: LetterMetadata, applicantName: string): string {
    const today = new Date().toLocaleDateString("de-DE");
    return `
${applicantName}
Empfänger: ${meta.senderAuthority}

Datum: ${today}
Aktenzeichen: ${meta.referenceNumber || "[Nicht angegeben]"}

Betreff: Antrag auf Fristverlängerung

Sehr geehrte Damen und Herren,

in vorbezeichneter Angelegenheit beziehe ich mich auf Ihr Schreiben vom ${meta.deadlineDate ? `mit Frist zum ${meta.deadlineDate}` : "letzter Korrespondenz"}.

Aufgrund von Verzögerungen bei der Beschaffung der erforderlichen Nachweise bitte ich hiermit höflich um Verlängerung der gesetzten Frist um 14 Tage.

Vielen Dank für Ihr Verständnis.

Mit freundlichen Grüßen,
${applicantName}
`.trim();
  }
}
