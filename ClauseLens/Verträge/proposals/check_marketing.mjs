import { pathToFileURL } from "node:url";
import fs from "node:fs";

const ROOT = "C:/Users/INTAL Admin/SYSTEMS & CREATIONS/ClauseLens";
const mod = await import(pathToFileURL(ROOT + "/src/analyze.js").href);
const { RULES, analyzeContract, flattenLineBreaks } = mod;

const P = (type, re, sev, evidence, risk, blocked) => ({ type, re, sev, evidence, risk, blocked: !!blocked });

const PROPOSALS = [
  // marketing_reference
  P("marketing_reference", /(?:f(?:ü|ue)hrt|nennt|bezeichnet|verwendet|nutzt|erw(?:ä|ae)hnt|darf)[^.!?;:\n]{0,70}?(?:den\s+|die\s+)?(?:auftraggeber|auftragnehmer|kunden|sponsor|lizenznehmer|lizenzgeber|partner|verein)[^.!?;:\n]{0,80}?(?:als\s+referenz\w*|als\s+kunden|in\s+(?:angeboten|fallstudien|case\s*studies|pressemitteilungen|marketingunterlagen|werbetr(?:ä|ae)gern|werbung|kundenlisten|prospekten))/i, "mittel", "führt/nennt ... Kunden ... als Referenz oder in Angeboten/Fallstudien", "medium"),
  P("marketing_reference", /(?:firmenname|bildmarke|marke|marken|logo)\b[^.!?;:\n]{0,70}?(?:in|auf)[^.!?;:\n]{0,25}?(?:werbung|werbetr(?:ä|ae)gern|marketingunterlagen|pressemitteilungen|angeboten|fallstudien|case\s*studies|website|kundenliste|prospekten)/i, "mittel", "Firmenname/Bildmarke erscheint in Werbung oder Marketingunterlagen", "medium"),
  P("marketing_reference", /(?:darf|d(?:ü|ue)rfen|ist\s+berechtigt|sind\s+berechtigt)[^.!?;:\n]{0,40}?(?:den\s+|die\s+)?(?:kunden|auftraggeber|lizenznehmer|sponsor|partner)[^.!?;:\n]{0,50}?als\s+referenz\w*/i, "mittel", "darf den Kunden als Referenz nennen", "medium"),
  // asymmetric_confidentiality
  P("asymmetric_confidentiality", /(?:nur|allein|ausschlie(?:ß|ss)lich)\s+(?:der|den|die|dem)?\s*(?:kunde|auftraggeber|besteller|mieter|sponsor|lizenznehmer|partner|creator|auftragnehmer)[^.!?;:\n]{0,80}?(?:vertraulich|geheimhaltung|verschwiegenheit|geheim)\w*/i, "mittel", "nur der Kunde ist zur Vertraulichkeit verpflichtet", "medium"),
  P("asymmetric_confidentiality", /(?:vertraulichkeits|geheimhaltungs|verschwiegenheits)pflicht[^.!?;:\n]{0,60}?(?:trifft\s+nur|obliegt\s+nur|nur\s+(?:den|der|dem))/i, "mittel", "Vertraulichkeitspflicht trifft nur den Auftraggeber", "medium"),
  P("asymmetric_confidentiality", /f(?:ü|ue)r\s+(?:den\s+|die\s+)?(?:anbieter|auftragnehmer|dienstleister|vermieter|lizenzgeber|agentur)[^.!?;:\n]{0,50}?(?:besteht\s+keine|gilt\s+(?:dies|das)\s+nicht|trifft\s+keine)/i, "mittel", "für den Anbieter gilt die Vertraulichkeit nicht", "medium"),
  P("asymmetric_confidentiality", /(?:den\s+|die\s+)?(?:anbieter|auftragnehmer|dienstleister)[^.!?;:\n]{0,40}?trifft\s+keine\s+(?:entsprechende\s+)?pflicht/i, "mittel", "den Anbieter trifft keine entsprechende Pflicht", "medium"),
  // excessive_survival
  P("excessive_survival", /(?:besteh\w+|gilt|gelten|wirkt|wirken|wird|werden|ist|sind)[^.!?;:\n]{0,60}?\b(?:auf\s+dauer|unbegrenzt|unbefristet|unver(?:ä|ae)ndert|dauerhaft|zeitlich\s+unbegrenzt)\b/i, "mittel", "gilt unbegrenzt / auf Dauer geschuldet", "medium"),
  P("excessive_survival", /(?:pflicht\w*|geheimhaltung\w*|verschwiegenheit\w*|rechte|anspr(?:ü|ue)che|provision|nachwirkung)[^.!?;:\n]{0,80}?(?:bestehen|besteht|gelten|gilt|wirken|wird\s+geschuldet|sind\s+geschuldet)[^.!?;:\n]{0,70}?(?:über\s+die\s+beendigung|nach\s+(?:vertragsende|beendigung)|hinaus)/i, "mittel", "Pflichten bestehen über die Beendigung hinaus", "medium"),
  P("excessive_survival", /(?:ü|ue)berdauern[^.!?;:\n]{0,70}?(?:beendigung|vertragsende|auslaufen|ende)/i, "mittel", "Rechte überdauern die Beendigung", "medium"),
  // setoff_restriction
  P("setoff_restriction", /(?:aufrechnung|verrechnung|einbehalt\w*|zur(?:ü|ue)ckbehaltung\w*|gegenforderung\w*|gegenanspruch|gegenanspr(?:ü|ue)che)[^.!?;:\n]{0,80}?(?:bedarf\s+der|nur\s+mit|ist\s+nicht\s+m(?:ö|oe)glich|sind\s+nicht\s+m(?:ö|oe)glich|ist\s+ausgeschlossen|sind\s+ausgeschlossen|bleiben[^.!?;:\n]{0,40}?(?:unber(?:ü|ue)cksichtigt|au(?:ß|ss)er\s+betracht)|kein\s+zur(?:ü|ue)ckbehaltungsrecht|begr(?:ü|ue)nden\s+kein|ist\s+kein\s+recht)/i, "mittel", "Aufrechnung bedarf der Zustimmung, Gegenforderungen bleiben außer Betracht", "medium"),
  P("setoff_restriction", /(?:kein\s+recht\s+(?:auf|zu|zum)\s+einbehalt|f(?:ü|ue)r\s+einbehalte\s+steht[^.!?;:\n]{0,40}?kein\s+recht)/i, "mittel", "für Einbehalte steht kein Recht zu", "medium"),
  P("setoff_restriction", /verzicht\w*[^.!?;:\n]{0,60}?(?:auf\s+)?(?:die\s+)?(?:aufrechnung|verrechnung|einbehalt|zur(?:ü|ue)ckbehaltung)/i, "mittel", "Kunde verzichtet auf die Aufrechnung", "medium"),
  // force_majeure_asymmetry
  P("force_majeure_asymmetry", /h(?:ö|oe)here\w?\s+gewalt[^.!?;:\n]{0,130}?(?:entbind\w+|entf(?:ä|ae)llt|befreit|freigestellt)[^.!?;:\n]{0,80}?(?:auftragnehmer|anbieter|dienstleister|veranstalter)/i, "hoch", "höhere Gewalt entbindet nur den Auftragnehmer", "medium"),
  P("force_majeure_asymmetry", /h(?:ö|oe)here\w?\s+gewalt[^.!?;:\n]{0,120}?(?:nur|ausschlie(?:ß|ss)lich)[^.!?;:\n]{0,80}?(?:leistungspflicht|verpflichtung|pflicht)[^.!?;:\n]{0,50}?(?:des|der)\s+(?:auftragnehmers|anbieters|dienstleisters)/i, "hoch", "entfällt nur die Leistungspflicht des Auftragnehmers", "medium"),
  // subcontractor_uncontrolled
  P("subcontractor_uncontrolled", /(?:darf|d(?:ü|ue)rfen|ist\s+berechtigt|sind\s+berechtigt|kann|k(?:ö|oe)nnen)[^.!?;:\n]{0,70}?(?:nachunternehmen|nachunternehmer|subunternehmer|medienpartner|partnerunternehmen|werbepl(?:ä|ae)tze|dritte)[^.!?;:\n]{0,60}?(?:einsetzen|weiterverkaufen|beauftragen|(?:ü|ue)bertragen|einbeziehen)/i, "mittel", "darf Nachunternehmer/Dritte nach eigenem Ermessen einsetzen", "medium"),
  P("subcontractor_uncontrolled", /(?:setzt|setzen|beauftragt|beauftragen|greift)[^.!?;:\n]{0,60}?(?:nachunternehmen|nachunternehmer|subunternehmer|medienpartner|dritte)[^.!?;:\n]{0,50}?(?:ohne\s+(?:vorherige\s+)?(?:ank(?:ü|ue)ndigung|zustimmung|information)|einer\s+zustimmung\s+bedarf\s+es\s+nicht)/i, "mittel", "setzt Medienpartner ohne Ankündigung ein", "medium"),
  P("subcontractor_uncontrolled", /(?:darf|d(?:ü|ue)rfen)[^.!?;:\n]{0,50}?ohne\s+(?:vorherige\s+)?(?:ank(?:ü|ue)ndigung|zustimmung|information)[^.!?;:\n]{0,70}?(?:nachunternehmen|nachunternehmer|subunternehmer|medienpartner|dritte)/i, "mittel", "darf ohne vorherige Ankündigung Nachunternehmer einsetzen", "medium"),
  // liability
  P("liability", /(?:haftung|schadensersatz|anspr(?:ü|ue)che)[^.!?;:\n]{0,80}?(?:ist|sind|werden|wird)[^.!?;:\n]{0,25}?auf\s+(?:die\s+|den\s+|das\s+|h(?:ö|oe)chstens\s+|maximal\s+)?[^.!?;:\n]{0,45}?(?:begrenzt|beschr(?:ä|ae)nkt|gedeckelt)/i, "mittel", "Haftung ist auf die Monatspauschale/5.000 EUR begrenzt", "medium"),
  P("liability", /(?:haftung|schadensersatz|anspr(?:ü|ue)che)[^.!?;:\n]{0,80}?(?:ist|sind|werden|wird)[^.!?;:\n]{0,25}?auf\s+(?:h(?:ö|oe)chstens\s+|maximal\s+)?\d[\d.,]*\s*(?:EUR|Euro|CHF)[^.!?;:\n]{0,20}?(?:begrenzt|beschr(?:ä|ae)nkt|gedeckelt)/i, "mittel", "Haftung ist auf 5.000 EUR begrenzt", "medium", true),
  P("liability", /(?:weitergehende|s(?:ä|ae)mtliche|alle)\s+anspr(?:ü|ue)che[^.!?;:\n]{0,80}?(?:sind|werden|ist)[^.!?;:\n]{0,60}?(?:abgegolten|abgefunden|erledigt)/i, "mittel", "weitergehende Ansprüche sind mit der Vergütung abgegolten", "medium"),
  // maintenance_conditional_shutdown
  P("maintenance_conditional_shutdown", /(?:mangels|ohne)\s+(?:abgeschlossenen\s+|g(?:ü|ue)ltigen\s+|bestehenden\s+)?wartungs(?:vertrag|vertrags|vereinbarung)\w*[^.!?;:\n]{0,80}?(?:wird|werden|darf|kann|ist|sind)[^.!?;:\n]{0,50}?(?:stillgelegt|abgeschaltet|au(?:ß|ss)er\s+betrieb|gesperrt|eingestellt|unterbrochen)/i, "mittel", "Mangels Wartungsvertrag wird die Anlage stillgelegt", "medium"),
  // price_escalation
  P("price_escalation", /(?:entgelt\w*|verg(?:ü|ue)tung\w*|pauschale\w*|miete\b|raummiete|geb(?:ü|ue)hr\w*|honorar\w*|preis\w*|monatspauschale)[^.!?;:\n]{0,60}?(?:steigt|steigen|erh(?:ö|oe)ht\s+sich|erh(?:ö|oe)hen\s+sich|wird\s+erh(?:ö|oe)ht|werden\s+erh(?:ö|oe)ht)[^.!?;:\n]{0,70}?(?:um\s+(?:\d|[a-zäöüß]{3,20})|j(?:ä|ae)hrlich|kalenderjahr|prozent)/i, "mittel", "Monatspauschale steigt mit jedem Kalenderjahr um fünf Prozent", "medium"),
  P("price_escalation", /sonderk(?:ü|ue)ndigungsrecht[^.!?;:\n]{0,30}?(?:besteht\s+nicht|ist\s+ausgeschlossen|wird\s+nicht\s+gew(?:ä|ae)hrt|ist\s+nicht\s+vorgesehen)/i, "mittel", "Sonderkündigungsrecht besteht nicht", "medium"),
  // entire_agreement
  P("entire_agreement", /(?:fr(?:ü|ue)here|bisherige|vorherige|m(?:ü|ue)ndliche)\s+(?:absprachen|vereinbarungen|zusagen|abreden|abmachungen)[^.!?;:\n]{0,60}?(?:treten|tritt|verlieren|verliert|werden|wird|sind)[^.!?;:\n]{0,50}?(?:au(?:ß|ss)er\s+kraft|unwirksam|gegenstandslos|aufgehoben|nicht\s+mehr\s+bestand)/i, "mittel", "frühere Absprachen treten außer Kraft", "medium"),
  // vicarious_agents
  P("vicarious_agents", /f(?:ü|ue)r\s+(?:das\s+verhalten|die\s+handlungen|handlungen)[^.!?;:\n]{0,60}?(?:beauftragter\s+dritter|dritter|unterlizenznehmer\w*|erf(?:ü|ue)llungsgehilfen|nachunternehmer\w*)[^.!?;:\n]{0,80}?wird\s+keine\s+(?:verantwortung|einstandspflicht|haftung)\b/i, "hoch", "für das Verhalten beauftragter Dritter wird keine Verantwortung übernommen", "medium"),
  P("vicarious_agents", /(?:verantwortung|einstandspflicht|haftung)[^.!?;:\n]{0,50}?f(?:ü|ue)r\s+(?:handlungen|das\s+verhalten)[^.!?;:\n]{0,60}?(?:beauftragter\s+)?dritter[^.!?;:\n]{0,60}?(?:wird\s+nicht|ist\s+nicht|keine)[^.!?;:\n]{0,30}?(?:ü|ue)bernommen/i, "hoch", "Eine Verantwortung für Handlungen Dritter wird nicht übernommen", "medium"),
  // assignment_restriction
  P("assignment_restriction", /(?:abtretung|(?:ü|ue)bertragung|weitergabe|weiterreichen|zession|abtreten)[^.!?;:\n]{0,80}?(?:bedarf|nur\s+mit|nur\s+nach|nicht\s+ohne)[^.!?;:\n]{0,40}?(?:zustimmung|einverst(?:ä|ae)ndnis|genehmigung|schriftlicher)/i, "mittel", "Übertragung von Rechten bedarf der Zustimmung", "medium"),
  P("assignment_restriction", /(?:kann|darf|k(?:ö|oe)nnen|d(?:ü|ue)rfen)[^.!?;:\n]{0,50}?anspr(?:ü|ue)che[^.!?;:\n]{0,70}?nur\s+mit\s+(?:dessen|deren|vorheriger|schriftlicher)\s+(?:einverst(?:ä|ae)ndnis|zustimmung|genehmigung)/i, "mittel", "kann Ansprüche nur mit Einverständnis weiterreichen", "medium"),
  // fictitious_receipt
  P("fictitious_receipt", /(?:erkl(?:ä|ae)rungen|mitteilungen|k(?:ü|ue)ndigungen|anzeigen|schreiben)[^.!?;:\n]{0,70}?gelten[^.!?;:\n]{0,60}?als\s+zugegangen/i, "mittel", "Erklärungen gelten mit Einstellung ins Portal als zugegangen", "medium"),
  P("fictitious_receipt", /(?:einstellung\s+in\s+das\s+kundenportal|abruf|versanddienstleister|absender(?:ü|ue)bergabe|(?:ü|ue)bergabe\s+an\s+den\s+versand)[^.!?;:\n]{0,70}?(?:kommt\s+es\s+nicht\s+an|als\s+zugegangen|gilt\s+als\s+zugang)/i, "mittel", "auf den Abruf kommt es nicht an", "medium"),
  // mandatory_expert_determination
  P("mandatory_expert_determination", /verfahrenskosten[^.!?;:\n]{0,60}?(?:kunde|besteller|auftraggeber|antragsteller)[^.!?;:\n]{0,40}?(?:vorzustrecken|vorauszuzahlen|zu\s+tragen)/i, "mittel", "Die Verfahrenskosten hat der Kunde vorzustrecken", "medium"),
  P("mandatory_expert_determination", /(?:rechtsweg|klage|gerichtlich)\w*[^.!?;:\n]{0,60}?erst\s+nach[^.!?;:\n]{0,60}?(?:abschluss|beendigung|durchf(?:ü|ue)hrung)[^.!?;:\n]{0,40}?(?:verfahrens|schiedsverfahrens|gutachtens)/i, "mittel", "Rechtsweg ist erst nach Abschluss des Verfahrens zulässig", "medium"),
  // non_compete
  P("non_compete", /nach\s+(?:dem\s+ende|beendigung|vertragsende|auslaufen|ablauf)[^.!?;:\n]{0,60}?(?:bleibt|ist|wird|sind|werden)[^.!?;:\n]{0,60}?(?:unt(?:ä|ae)tig|nicht\s+(?:t(?:ä|ae)tig|beratend|werblich|vermittelnd|konkurrierend)|nicht\s+mehr\s+t(?:ä|ae)tig)/i, "mittel", "Nach dem Ende der Zusammenarbeit bleibt der Creator untätig", "medium"),
  P("non_compete", /f(?:ü|ue)r\s+(?:wettbewerber|konkurrenz|konkurrierende|wettbewerbliche|andere\s+marken)[^.!?;:\n]{0,60}?(?:nicht\s+(?:t(?:ä|ae)tig|beratend|werblich|vermittelnd|konkurrierend)|unt(?:ä|ae)tig)/i, "mittel", "für Wettbewerber nicht beratend tätig", "medium"),
  // default_interest
  P("default_interest", /(?:zinsen|zinssatz|verzinsung|verzugszins\w*)[^.!?;:\n]{0,60}?(?:\d{1,2}(?:[.,]\d+)?\s*%|[a-zäöüß]{3,20}\s+prozent|prozent)[^.!?;:\n]{0,40}?(?:monatlich|pro\s+monat|je\s+monat|w(?:ö|oe)chentlich|t(?:ä|ae)glich)/i, "hoch", "Zinsen von acht Prozent monatlich", "medium", true),
  // service_suspension
  P("service_suspension", /(?:sperrt|sperren|unterbricht|unterbrechen|stellt|stellen|setzt|setzen|stoppt|stoppen)[^.!?;:\n]{0,60}?(?:den\s+|die\s+|das\s+)?(?:zugang|betreuung|leistung|plakatierung|lieferung|service|belieferung)[^.!?;:\n]{0,60}?(?:unverz(?:ü|ue)glich|ohne\s+(?:vorank(?:ü|ue)ndigung|abmahnung)|fristlos|sofort|jederzeit|ohne\s+grund)/i, "hoch", "sperrt den Zugang unverzüglich", "medium"),
  P("service_suspension", /(?:zugang|betreuung|leistung|plakatierung|lieferung|service|belieferung)[^.!?;:\n]{0,60}?(?:ohne\s+(?:vorank(?:ü|ue)ndigung|abmahnung)|fristlos|sofort)[^.!?;:\n]{0,40}?(?:sperren|unterbrechen|einstellen|aussetzen|stoppen)/i, "hoch", "Betreuung ohne Vorankündigung unterbrechen", "medium"),
  // deemed_acceptance
  P("deemed_acceptance", /m(?:ä|ae)ngel\w*[^.!?;:\n]{0,50}?nicht\s+binnen[^.!?;:\n]{0,40}?(?:ger(?:ü|ue)gt|angezeigt|gemeldet|beanstandet)[^.!?;:\n]{0,70}?gilt[^.!?;:\n]{0,40}?als\s+(?:vertragsgem(?:ä|ae)(?:ß|ss)|abgenommen|genehmigt|freigegeben)/i, "mittel", "Mängel nicht binnen achtundvierzig Stunden gerügt, gilt als vertragsgemäß", "medium"),
  P("deemed_acceptance", /(?:schweigt|schweigen|widerspricht\s+nicht|erhebt\s+keine\s+einw(?:ä|ae)nde)[^.!?;:\n]{0,70}?gilt[^.!?;:\n]{0,40}?als\s+(?:freigegeben|abgenommen|genehmigt|vertragsgem(?:ä|ae)(?:ß|ss))/i, "mittel", "Schweigt der Auftraggeber, gilt das Material als freigegeben", "medium"),
  // statute_of_limitations
  P("statute_of_limitations", /(?:verj(?:ä|ae)hr\w*|verwirk\w*)[^.!?;:\n]{0,60}?(?:\d{1,2}|ein|eine|einer|zwei|drei|sechs|zw(?:ö|oe)lf|achtzehn|vierundzwanzig)\s*(?:monat\w*|jahr\w*|woche\w*|tag\w*)/i, "mittel", "Mängelansprüche verjähren ein Jahr nach Lieferung", "medium"),
  // burden_of_proof
  P("burden_of_proof", /nachweis\w*[^.!?;:\n]{0,80}?hat\s+(?:der|die|das)\s+(?:kunde|auftraggeber|besteller|k(?:ä|ae)ufer|mieter|auftragnehmer)[^.!?;:\n]{0,40}?zu\s+f(?:ü|ue)hren/i, "hoch", "Den Nachweis hat der Auftraggeber zu führen", "medium"),
  P("burden_of_proof", /nachweis\w*[^.!?;:\n]{0,60}?obliegt\s+dem\s+(?:kunden|auftraggeber|besteller|k(?:ä|ae)ufer)/i, "hoch", "Nachweis obliegt dem Kunden", "medium"),
  // feedback_ip_grab
  P("feedback_ip_grab", /(?:verbesserungsvorschl(?:ä|ae)ge|anregungen|ideen|r(?:ü|ue)ckmeldungen)[^.!?;:\n]{0,70}?(?:werden|gehen|sind|wird)[^.!?;:\n]{0,60}?(?:unentgeltlich\s+)?(?:eigentum|bestandteil)[^.!?;:\n]{0,40}?(?:des|der)\s+(?:auftragnehmers|anbieters|dienstleisters|agentur)/i, "mittel", "Verbesserungsvorschläge werden unentgeltlich Eigentum des Auftragnehmers", "medium"),
  // unilateral_change
  P("unilateral_change", /(?:einseitig\w*|nach\s+billigem\s+ermessen)[^.!?;:\n]{0,80}?(?:(?:ä|ae)nder\w*|angleich\w+|anpass\w+|einschr(?:ä|ae)nk\w+)/i, "mittel", "Leistungsumfang nach billigem Ermessen angleichen", "medium"),
  P("unilateral_change", /(?:(?:ä|ae)nder\w*|angleich\w+|anpass\w+|einschr(?:ä|ae)nk\w+)[^.!?;:\n]{0,60}?(?:einseitig|nach\s+billigem\s+ermessen|ohne\s+zustimmung)/i, "mittel", "ändert den Redaktionsplan einseitig", "medium"),
  // agency_commission_exclusion
  P("agency_commission_exclusion", /direkt(?:verk(?:ä|ae)ufe|auftr(?:ä|ae)ge|gesch(?:ä|ae)ft\w*|verkauf\w*)[^.!?;:\n]{0,80}?(?:l(?:ö|oe)sen|l(?:ö|oe)st)[^.!?;:\n]{0,30}?kein\w*\s+(?:provision|provisionsanspruch|verg(?:ü|ue)tung|anspruch)/i, "hoch", "Direktaufträge lösen keinen Provisionsanspruch aus", "medium"),
  // agency_goodwill_waiver
  P("agency_goodwill_waiver", /ausgleich\w*[^.!?;:\n]{0,80}?(?:ist|wird|sind|werden)[^.!?;:\n]{0,30}?(?:ausgeschlossen|abbedungen|nicht\s+geschuldet|verzichtet)/i, "hoch", "Ausgleichsanspruch nach Vertragsende ist ausgeschlossen", "medium"),
  P("agency_goodwill_waiver", /(?:verzicht\w*|abbedungen|ausgeschlossen)[^.!?;:\n]{0,60}?(?:auf\s+)?(?:den\s+)?ausgleich\w*/i, "hoch", "Verzicht auf den Ausgleich", "medium"),
  // non_solicitation
  P("non_solicitation", /(?:stellt|stellen|wird|werden|darf|d(?:ü|ue)rfen)[^.!?;:\n]{0,70}?keine\s+(?:ehemaligen|fr(?:ü|ue)heren|aktuellen)?\s*(?:mitarbeiter|besch(?:ä|ae)ftigte)[^.!?;:\n]{0,40}?ein\b/i, "mittel", "stellt keine ehemaligen Mitarbeiter des Kunden ein", "medium"),
  // unreasonable_audit
  P("unreasonable_audit", /(?:pr(?:ü|ue)fung\w*|audit\w*|kontroll\w*|einsicht\w*|zutritt)[^.!?;:\n]{0,80}?(?:ohne\s+(?:anlass|vorank(?:ü|ue)ndigung|terminabstimmung)|anlasslos|jederzeit)/i, "hoch", "Prüfungen ohne Anlass durchführen", "medium"),
  // dynamic_url_referral
  P("dynamic_url_referral", /gelten\s+die[^.!?;:\n]{0,80}?(?:abrufbar\w*|hinterlegt\w*|jeweils\s+g(?:ü|ue)ltig\w*|jeweils\s+aktuell\w*)[^.!?;:\n]{0,60}?(?:fassung\w*|bedingung\w*|richtlinie\w*|version\w*|einkaufsbedingungen)/i, "hoch", "gelten die im Kundenportal hinterlegten Einkaufsbedingungen", "medium"),
  P("dynamic_url_referral", /(?:abrufbaren|hinterlegten|jeweils\s+g(?:ü|ue)ltigen)\s+(?:fassungen|versionen|bedingungen|richtlinien|einkaufsbedingungen)/i, "hoch", "die abrufbaren Fassungen der Werberichtlinien", "medium"),
  // as_is_software_disclaimer
  P("as_is_software_disclaimer", /(?:ohne\s+m(?:ä|ae)ngelgew(?:ä|ae)hr|im\s+ist-zustand|wie\s+gesehen)[^.!?;:\n]{0,80}?(?:ohne|und\s+ohne|keine)\s+(?:jegliche\s+)?(?:zusicherung|gew(?:ä|ae)hrleistung|garantie|m(?:ä|ae)ngelhaftung)/i, "hoch", "ohne Mängelgewähr und ohne Zusicherung der Marktgängigkeit", "medium"),
  P("as_is_software_disclaimer", /(?:ohne\s+m(?:ä|ae)ngelgew(?:ä|ae)hr|wie\s+gesehen|im\s+ist-zustand)[^.!?;:\n]{0,90}?(?:überlassen|bereitgestellt|geliefert|verkauft|erworben)/i, "hoch", "Inhalte werden ohne Mängelgewähr überlassen", "medium"),
  // written_form_trap
  P("written_form_trap", /m(?:ü|ue)ndliche\s+(?:absprachen|abreden|vereinbarungen|zusagen)[^.!?;:\n]{0,70}?(?:sind|ist|bleiben|entfalten)[^.!?;:\n]{0,40}?(?:ohne\s+rechtswirkung|unwirksam|unverbindlich|keine\s+wirkung)/i, "mittel", "mündliche Absprachen sind ohne Rechtswirkung", "medium"),
  P("written_form_trap", /(?:nur\s+g(?:ü|ue)ltig|nur\s+wirksam|nur\s+in\s+schriftform)[^.!?;:\n]{0,60}?in\s+(?:papierform|schriftform)[^.!?;:\n]{0,60}?(?:originalunterschrift|eigenh(?:ä|ae)ndig|handschriftlich)/i, "mittel", "Änderungen nur in Papierform mit Originalunterschrift", "medium"),
];

// ---------- dossier ----------
const raw = fs.readFileSync(ROOT + "/Verträge/proposals/misses_marketing.txt", "utf8");
const entries = [];
const blocks = raw.split(/^### /m).slice(1);
for (const b of blocks) {
  const head = b.match(/^([a-z0-9-]+):L(\d+)\s+type=(\w+)/i);
  if (!head) continue;
  const lines = b.split(/\r?\n/);
  const para = lines.slice(1).filter((l) => l.trim() && !/^note:/.test(l)).join(" ").trim();
  entries.push({ slug: head[1], line: Number(head[2]), type: head[3], para });
}
console.log("dossier entries:", entries.length);

const norm = (s) => s.replace(/\s+/g, " ").trim().toLowerCase();
const hasFinding = (findings, type, marker) => findings.some((f) => f.type === type && norm(f.excerpt).includes(norm(marker)));
const firesOn = (text, type) => analyzeContract(text).findings.some((f) => f.type === type);

// ---------- baseline ----------
const baseEntries = entries.filter((e) => firesOn(e.para, e.type)).length;
const contracts = [];
for (let i = 1; i <= 32; i++) {
  const slug = "marketing-" + String(i).padStart(3, "0");
  contracts.push({ slug, text: fs.readFileSync(ROOT + "/Verträge/corpus/" + slug + ".txt", "utf8"), label: JSON.parse(fs.readFileSync(ROOT + "/Verträge/corpus/labels/" + slug + ".json", "utf8")) });
}
function trapRecall() {
  let traps = 0, hit = 0;
  for (const c of contracts) {
    const fs2 = analyzeContract(c.text).findings;
    for (const t of c.label.traps) { traps++; if (hasFinding(fs2, t.expectedType, t.marker)) hit++; }
  }
  return { traps, hit };
}
const baseRecall = trapRecall();
const benignBase = [];
const benignKeys = (c, b, fs2) => fs2.filter((f) => norm(f.excerpt).includes(norm(b.marker))).map((f) => c.slug + "::" + b.marker + "::" + f.type);
for (const c of contracts) {
  const fs2 = analyzeContract(c.text).findings;
  for (const b of c.label.benign) benignBase.push(...benignKeys(c, b, fs2));
}
console.log("BASELINE entries-fired:", baseEntries + "/" + entries.length, "| trap recall", baseRecall.hit + "/" + baseRecall.traps, "| benign FPs", benignBase.length);

// ---------- patch ----------
const byType = new Map();
for (const p of PROPOSALS) {
  if (!byType.has(p.type)) byType.set(p.type, []);
  byType.get(p.type).push(p);
}
const patchedRules = [];
for (const rule of RULES) {
  const props = byType.get(rule.type);
  if (!props) continue;
  patchedRules.push(rule.type);
  if (typeof rule.collect === "function") {
    const orig = rule.collect;
    rule.collect = function (sentences, text) {
      const hits = orig.call(this, sentences, text) || [];
      const seen = new Set(hits.map((h) => h.i));
      const extra = [];
      sentences.forEach((s, i) => {
        if (seen.has(i)) return;
        const t = flattenLineBreaks(s.text);
        for (const p of props) if (p.re.test(t)) { extra.push({ i, severity: p.sev }); break; }
      });
      return hits.concat(extra);
    };
  } else {
    for (const p of props) rule.patterns = (rule.patterns || []).concat([new RegExp(p.re.source, p.re.flags)]);
  }
}
console.log("patched rule types:", patchedRules.join(","));

// ---------- per-entry ----------
const closes = new Map();
const stillMissed = [];
for (const e of entries) {
  const ok = firesOn(e.para, e.type);
  const key = e.slug + ":L" + e.line;
  if (!ok) { stillMissed.push(key + " " + e.type); continue; }
  for (const p of byType.get(e.type) || []) {
    if (p.re.test(flattenLineBreaks(e.para)) || analyzeContract(e.para).findings.some((f) => f.type === e.type && p.re.test(flattenLineBreaks(f.excerpt)))) {
      if (!closes.has(p)) closes.set(p, []);
      closes.get(p).push(key);
      break;
    }
  }
}
console.log("CLOSED:", entries.length - stillMissed.length + "/" + entries.length);
console.log("STILL MISSED:", JSON.stringify(stillMissed));
const blocked = [];
for (const e of entries) {
  const key = e.slug + ":L" + e.line;
  if (!stillMissed.some((m) => m.startsWith(key))) continue;
  if ((byType.get(e.type) || []).some((p) => p.re.test(flattenLineBreaks(e.para)))) blocked.push(key + " " + e.type);
}
console.log("PATTERN-OK-SEVERITY-BLOCKED:", JSON.stringify(blocked));

// ---------- after ----------
const afterRecall = trapRecall();
const benignAfter = [];
const proposalHits = [];
for (const c of contracts) {
  const fs2 = analyzeContract(c.text).findings;
  for (const b of c.label.benign) benignAfter.push(...benignKeys(c, b, fs2));
}
for (const c of contracts) for (const b of c.label.benign) for (const p of PROPOSALS) {
  if (p.re.test(flattenLineBreaks(b.marker))) { proposalHits.push(p.type + " :: " + c.slug + " :: " + b.marker); break; }
}
const baseSet = new Set(benignBase);
const newFps = benignAfter.filter((k) => !baseSet.has(k));
console.log("AFTER trap recall", afterRecall.hit + "/" + afterRecall.traps);
console.log("proposal regex hits on benign markers:", proposalHits.length, "| NEW benign findings vs baseline:", newFps.length, "| pre-existing engine benign FPs:", baseSet.size);
if (proposalHits.length) console.log("FP-PROPOSAL", JSON.stringify(proposalHits, null, 1));
if (newFps.length) console.log("FP-NEW", JSON.stringify(newFps, null, 1));
console.log("FP-PREEXISTING", JSON.stringify([...baseSet], null, 1));

// ---------- emit ----------
const out = { slice: "marketing", patterns: [] };
for (const p of PROPOSALS) {
  const cl = closes.get(p) || [];
  if (!cl.length && !p.blocked) continue;
  out.patterns.push({ type: p.type, regex: p.re.source, flags: p.re.flags, closes: cl.sort(), evidence: p.blocked ? "severity-blocked: " + p.evidence : p.evidence, risk: p.risk });
}
fs.writeFileSync(ROOT + "/Verträge/proposals/patterns_marketing.json", JSON.stringify(out, null, 2) + "\n");
console.log("wrote patterns:", out.patterns.length, "| closes total", out.patterns.reduce((a, p) => a + p.closes.length, 0));
