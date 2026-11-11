export interface LetterMetadata {
  senderAuthority: string;
  referenceNumber: string | null;
  taxId: string | null;
  socialSecurityNumber: string | null;
  documentType: string;
  deadlineDate: string | null;
  daysRemaining: number | null;
  actionRequired: string;
  actionList: string[];
  urgencyLevel: "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
  simpleSummary: string;
  detailedExplanation: string;
  isActionRequired: boolean;
}

export class LetterExtractor {
  public static extractMetadata(rawOcrText: string, currentDate: Date = new Date()): LetterMetadata {
    const text = rawOcrText.trim();
    if (!text) {
      return {
        senderAuthority: "Unbekannt",
        referenceNumber: null,
        taxId: null,
        socialSecurityNumber: null,
        documentType: "Nicht lesbar",
        deadlineDate: null,
        daysRemaining: null,
        actionRequired: "Foto erneut aufnehmen",
        actionList: ["Foto bei besserer Beleuchtung und ohne Schatten wiederholen"],
        urgencyLevel: "INFO",
        simpleSummary: "Es konnte kein lesbarer Text erkannt werden.",
        detailedExplanation: "Bitte fotografieren Sie den Brief gerade von oben und achten Sie darauf, dass alle Ecken und Texte gut ausgeleuchtet sind.",
        isActionRequired: false
      };
    }

    // 1. Specific Entity Extractions
    // Sozialversicherungsnummer (RVNR): e.g. 12 150896 M 014 or 65120590J123
    let socialSecurityNumber: string | null = null;
    const svnMatch = text.match(/(?:Versicherungsnummer|Sozialversicherungsnummer|RVNR|Rentenversicherungsnummer)[\s:]*([0-9]{2}[\s\-]?[0-9]{6}[\s\-]?[A-Z][\s\-]?[0-9]{3})/i)
      || text.match(/\b([0-9]{2}\s[0-9]{6}\s[A-Z]\s[0-9]{3})\b/i);
    if (svnMatch) {
      socialSecurityNumber = svnMatch[1].replace(/[\s\-]/g, " ");
    }

    // Steuer-ID (11-stellig): e.g. 42 819 304 912
    let taxId: string | null = null;
    const taxMatch = text.match(/(?:Identifikationsnummer|IdNr\.?|Steuer-ID|Steuer-Identifikationsnummer)[\s:]*([0-9]{2}[\s\-]?[0-9]{3}[\s\-]?[0-9]{3}[\s\-]?[0-9]{3})/i)
      || text.match(/\b([0-9]{2}\s[0-9]{3}\s[0-9]{3}\s[0-9]{3})\b/);
    if (taxMatch) {
      taxId = taxMatch[1].replace(/[\s\-]/g, " ");
    }

    // Aktenzeichen / Geschäftszeichen
    let referenceNumber: string | null = null;
    const refMatch = text.match(/(?:Aktenzeichen|Az\.?|Geschäftszeichen|Mein Zeichen|Gesch.-Z.|BG-Nummer|Kundennummer|Beitragsnummer)[\s:]*([A-Za-z0-9\-\.\/]+)/i);
    if (refMatch) {
      referenceNumber = refMatch[1];
    } else if (socialSecurityNumber) {
      referenceNumber = `VSNR: ${socialSecurityNumber}`;
    } else if (taxId) {
      referenceNumber = `IdNr: ${taxId}`;
    }

    // 2. Comprehensive German Document Intelligence
    let sender = "Behörde / Institution";
    let docType = "Amtliches Schreiben";
    let isInformationalOnly = false;
    let summary = "";
    let detailed = "";
    const actions: string[] = [];

    // --- CASE A: Versicherungsnummernachweis / Sozialversicherungsausweis ---
    if (socialSecurityNumber || /versicherungsnummernachweis|sozialversicherungsausweis|rentenversicherung.*versicherungsnummer|sozialversicherung.*nachweis/i.test(text)) {
      sender = "Deutsche Rentenversicherung (DRV)";
      docType = "Versicherungsnummernachweis (Sozialversicherung)";
      isInformationalOnly = true;
      summary = `Dieser Brief ist Ihr offizieller Nachweis über Ihre deutsche Sozialversicherungsnummer${socialSecurityNumber ? ' (' + socialSecurityNumber + ')' : ''}. Er bestätigt Ihre Registrierung im Renten- und Sozialversicherungssystem.`;
      detailed = "Sie erhalten dieses Schreiben bei Aufnahme Ihrer ersten Beschäftigung in Deutschland oder bei Neuausstellung. Es ersetzt den früheren Sozialversicherungsausweis. Sie müssen dem Amt nichts antworten.";
      actions.push("Diesen Nachweis bei Aufnahme einer neuen Arbeit dem Arbeitgeber vorlegen");
      actions.push("Nummer für Anträge bei Krankenkasse, Arbeitsagentur oder Rente bereithalten");
      actions.push("Dokument dauerhaft und sicher bei Ihren persönlichen Unterlagen aufbewahren");
    }

    // --- CASE B: Steueridentifikationsnummer (BZSt) ---
    else if (taxId || /steueridentifikationsnummer|identifikationsnummer|idnr|bundeszentralamt f[üu]r steuern|bzst/i.test(text)) {
      sender = "Bundeszentralamt für Steuern (BZSt)";
      docType = "Zuteilung der Steuer-Identifikationsnummer (IdNr)";
      isInformationalOnly = true;
      summary = `Dieser Brief enthält Ihre lebenslange persönliche Steuer-Identifikationsnummer${taxId ? ' (' + taxId + ')' : ''}.`;
      detailed = "Die Steuer-ID bleibt Ihr ganzes Leben lang gültig und ändert sich auch bei Umzügen oder Heirat nicht. Es ist keine Rücksendung oder Antwort an das Bundeszentralamt für Steuern erforderlich.";
      actions.push("Steuer-ID dem Arbeitgeber für die Lohnabrechnung mitteilen");
      actions.push("Steuer-ID bei der Bank für Freistellungsaufträge angeben");
      actions.push("Brief sorgfältig zu den Steuerunterlagen ablegen");
    }

    // --- CASE C: Rundfunkbeitrag (ARD ZDF Deutschlandradio) ---
    else if (/rundfunkbeitrag|beitragsservice|ard zdf deutschlandradio|gebühreneinzugszentrale/i.test(text)) {
      sender = "ARD ZDF Deutschlandradio Beitragsservice";
      docType = "Rundfunkbeitrag / Zahlungsaufforderung";
      summary = "Dieser Brief betrifft den gesetzlichen Rundfunkbeitrag für Ihre Wohnung (18,36 € pro Monat).";
      detailed = "In Deutschland muss grundsätzlich jede volljährige Person bzw. jeder Haushalt Rundfunkbeitrag zahlen, es sei denn, es liegt eine Befreiung vor (z. B. bei Bezug von Bürgergeld oder BAföG).";
      if (/befreiung|bafög|bürgergeld/i.test(text)) {
        actions.push("Befreiungsantrag ausfüllen und Nachweis (z.B. Bürgergeldbescheid) beilegen");
      } else {
        actions.push("Beitragsnummer prüfen und ggf. Überweisung tätigen oder SEPA-Lastschrift einrichten");
      }
      actions.push("Falls bereits ein Mitbewohner zahlt: Dessen Beitragsnummer mitteilen");
    }

    // --- CASE D: Amtliche Meldebestätigung ---
    else if (/meldebest[aä]tigung|anmeldung bei der meldebeh[öo]rde|wohnsitzanmeldung/i.test(text)) {
      sender = "Bürgeramt / Einwohnermeldeamt";
      docType = "Amtliche Meldebestätigung";
      isInformationalOnly = true;
      summary = "Dies ist Ihre offizielle Bestätigung über Ihren Wohnsitz in Deutschland.";
      detailed = "Die Meldebestätigung dient als gesetzlicher Nachweis Ihrer Adresse für Banken, Mobilfunkverträge, Behörden und die Eröffnung von Konten. Sie müssen nichts an das Bürgeramt zurücksenden.";
      actions.push("Dokument sicher aufbewahren (wird oft als Adressnachweis verlangt)");
      actions.push("Bei Banken, Krankenkasse und Arbeitgeber als Adressnachweis vorlegen");
    }

    // --- CASE E: Jobcenter Mitwirkungsaufforderung ---
    else if (/jobcenter|agentur f[üu]r arbeit/i.test(text) && /mitwirkung|nachweis|unterlagen/i.test(text)) {
      sender = "Jobcenter / Agentur für Arbeit";
      docType = "Aufforderung zur Mitwirkung";
      summary = "Das Jobcenter fordert fehlende Nachweise an, um Ihre Leistungen zu berechnen.";
      detailed = "Wenn Sie die geforderten Dokumente nicht rechtzeitig vor Fristablauf einreichen, können Ihre Geldleistungen (Bürgergeld/Miete) vorübergehend ganz oder teilweise gestrichen werden.";
      if (/kontoausz[üu]g/i.test(text)) actions.push("Kontoauszüge der letzten 3 Monate lückenlos einreichen");
      if (/mietbescheinigung/i.test(text)) actions.push("Mietbescheinigung vom Vermieter ausfüllen lassen und einreichen");
      if (/gehaltsabrechnung|einkommen/i.test(text)) actions.push("Einkommensnachweise / Lohnabrechnungen vorlegen");
      if (actions.length === 0) actions.push("Die im Brief markierten Nachweise vor Fristende einreichen");
    }

    // --- CASE F: Finanzamt Steuererinnerung ---
    else if (/finanzamt/i.test(text) && /steuererkl[aä]rung|erinnerung/i.test(text)) {
      sender = "Finanzamt";
      docType = "Erinnerung zur Abgabe der Steuererklärung";
      summary = "Das Finanzamt erinnert Sie an die Abgabe Ihrer noch ausstehenden Steuererklärung.";
      detailed = "Bitte reichen Sie Ihre Steuererklärung fristgerecht ein. Bei Fristüberschreitung kann das Finanzamt Verspätungszuschläge festsetzen oder Ihre Einkünfte schätzen.";
      actions.push("Steuererklärung über ELSTER oder Steuersoftware übermitteln");
      actions.push("Falls Belege fehlen: Schriftlich Fristverlängerung beim Finanzamt beantragen");
    }

    // --- CASE G: Ausländerbehörde Anhörung ---
    else if (/ausl[aä]nderbeh[öo]rde|einwanderung|lea|landesamt f[üu]r einwanderung/i.test(text)) {
      sender = "Ausländerbehörde (LEA)";
      docType = "Aufenthaltsangelegenheit / Anhörung";
      summary = "Die Ausländerbehörde informiert Sie über Ihren Aufenthaltsstatus oder fordert Nachweise an.";
      detailed = "Achten Sie unbedingt auf Fristen, da das Versäumen von Terminen oder Nachweisen Ihren Aufenthaltsstatus oder die Arbeitserlaubnis gefährden kann.";
      actions.push("Geforderte Unterlagen (Pass, Arbeitsvertrag, Gehaltsabrechnung) zusammenstellen");
      actions.push("Fristgerecht schriftlich antworten oder Termin wahrnehmen");
    }

    // --- DEFAULT FALLBACK (Dynamic text-based) ---
    else {
      sender = "Behörde / Institution";
      docType = "Amtliche Mitteilung";
      summary = "Dieses Schreiben informiert Sie über einen Vorgang bei der genannten Behörde.";
      detailed = "Prüfen Sie den Inhalt auf persönliche Fristen oder Zahlungsaufforderungen.";
      actions.push("Schreiben sorgfältig prüfen und bei den persönlichen Akten abheften");
    }

    // 3. Strict Deadline Check
    let deadlineDate: string | null = null;
    let daysRemaining: number | null = null;

    if (!isInformationalOnly) {
      const fristMatch = text.match(/(?:Frist bis|bis zum|sp[aä]testens am|sp[aä]testens bis|Ablauf der Frist)[\s:]*([0-3]?[0-9]\.[0-1]?[0-9]\.20[2-3][0-9])/i)
        || text.match(/(?:bis\s+zum|zum)\s*([0-3]?[0-9]\.[0-1]?[0-9]\.20[2-3][0-9])/i);

      if (fristMatch) {
        deadlineDate = fristMatch[1];
        const parts = deadlineDate.split(".");
        const d = parseInt(parts[0], 10);
        const m = parseInt(parts[1], 10);
        const y = parseInt(parts[2], 10);
        const parsedDeadline = new Date(y, m - 1, d);
        const diffMs = parsedDeadline.getTime() - currentDate.getTime();
        const calcDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));
        daysRemaining = calcDays >= 0 ? calcDays : 0;
      }
    }

    // 4. Urgency
    let urgency: LetterMetadata["urgencyLevel"] = "INFO";
    if (daysRemaining !== null) {
      if (daysRemaining <= 3) urgency = "CRITICAL";
      else if (daysRemaining <= 7) urgency = "HIGH";
      else if (daysRemaining <= 14) urgency = "MEDIUM";
      else urgency = "LOW";
    }

    return {
      senderAuthority: sender,
      referenceNumber,
      taxId,
      socialSecurityNumber,
      documentType: docType,
      deadlineDate,
      daysRemaining,
      actionRequired: actions[0] || "Abheften",
      actionList: actions,
      urgencyLevel: urgency,
      simpleSummary: summary,
      detailedExplanation: detailed,
      isActionRequired: !isInformationalOnly && (deadlineDate !== null || urgency !== "INFO")
    };
  }
}
