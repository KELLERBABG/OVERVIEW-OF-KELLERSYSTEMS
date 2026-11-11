/* AmtFlow Document Classifier
 * Data-driven, keyword-weighted classifier for German authority, bank,
 * insurance, utility, education and identity documents.
 * UMD: works as window.AmtFlowClassifier (browser) and module.exports (node).
 */
(function (root, factory) {
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = factory();
  } else {
    root.AmtFlowClassifier = factory();
  }
})(typeof self !== 'undefined' ? self : this, function () {

  // --- Document type catalog -------------------------------------------------
  // Each rule: { key, w: weight, p: [patterns] }. A pattern is a string that
  // is regex-escaped and matched case-insensitively, or a RegExp.
  const RULES = [// Specific subtypes with appropriate weights
    { key: 'jobcenter_weiterbewilligung', w: 64, p: [/weiterbewilligungsantrag|weiterbewilligung.*(sgb|b[üu]rgergeld|leistungen)|wba.*jobcenter|folgeantrag.*b[üu]rgergeld/] },
    { key: 'jobcenter_bewilligung', w: 64, p: [/b[üu]rgergeld[-_ ]+bewilligungsbescheid|bewilligungsbescheid.*b[üu]rgergeld|b[üu]rgergeld.*bewillig|bewilligungsbescheid.*sgb[-_ ]?ii\b|jobcenter.*bewilligungsbescheid/] },
    
    { key: 'renten_kontoklarung', w: 55, p: [/kontenkl[äa]rung|kontenkl[äa]rungsverfahren|kontenkl[äa]rungsbogen|kontokl[äa]rung.*rente/] },
    { key: 'vollstreckungsbescheid', w: 56, p: [/vollstreckungsbescheid|vollstreckbarer betrag|vollstreckungstitel.*mahnverfahren/] },

    // Employment, vocational training & company contracts
    { key: 'ausbildungsvertrag', w: 55, p: [/berufsausbildungsvertrag|ausbildungsvertrag|lehrvertrag|ausbildungsverh[äa]ltnis|ausbildungsbetrieb.*vertrag|vertrag.*berufsausbildung|ihk.*ausbildungsvertrag|hwk.*ausbildungsvertrag|ausbildungsordnung|ausbildungsverg[üu]tung/] },
    { key: 'leihvertrag', w: 55, p: [/leihvertrag|sachleihvertrag|[üu]berlassungsvertrag|leihvereinbarung|[üu]berlassung.*arbeitsmittel|leihger[äa]t|leihschein|leihgabe.*betrieb|dienstger[äa]t.*leih/] },
    { key: 'arbeitsvertrag', w: 52, p: [/arbeitsvertrag|anstellungsvertrag|dienstvertrag.*arbeit|besch[äa]ftigungsvertrag|arbeitsverh[äa]ltnis.*vertrag/] },
    { key: 'leiharbeit_vertrag', w: 52, p: [/arbeitnehmer[üu]berlassung|zeitarbeitsvertrag|leiharbeitsvertrag|personaldienstleist.*vertrag|entleihung/] },
    { key: 'praktikumsvertrag', w: 52, p: [/praktikumsvertrag|praktikantenvertrag|pflichtpraktikum.*vertrag|praktikum.*vereinbarung/] },
    // Identity & online-ID
    { key: 'online_ausweis', w: 65, p: [/online[- ]?ausweis|ausweisfunktion|transport[- ]?pin|sperrkennwort|pin[- ]?brief.*ausweis|personalausweis.*pin|bundesdruckerei|\beid\b|perso.*pin/] },
    { key: 'fuehrungszeugnis', w: 40, p: [/f[iu]hrungszeugnis|bundesamt f[iu]r justiz|zentralregister|eintragung (?:keine|frei)/] },
    { key: 'ausweis_abholung', w: 38, p: [/zur abholung|abholung bereit|liegt zur abholung|ausweis.*abholen|pass.*abholung|abholbenachrichtigung/] },

    // Pensions & social insurance
    { key: 'rvnr', w: 45, p: [/versicherungsnummernachweis|sozialversicherungsausweis|sozialversicherungsnummer|\bvsnr\b|bereichsnummer|rentenversicherungsnummer/] },
    { key: 'renteninfo', w: 36, p: [/renteninformation|rentenauskunft|altersrente|erwerbsminderungsrente|rentenanspruch/] },
    { key: 'renten_berechnungshinweis', w: 48, p: [/berechnungshinweis.*rente|rente.*berechnungshinweis|hinweis.*rentenberechnung|rente.*wird.*berechnet/] },
    { key: 'rentenbescheid', w: 42, p: [/rentenbescheid|rente wird festgesetzt|rentenberechnung|z[üu]schlag.*rente|rente.*bewilligt/] },
    { key: 'rentenantrag', w: 38, p: [/rentenantrag|antrag.*rente|rente.*beantragt|rentenantrag.*stellen/] },
    { key: 'renten_kontoklarung', w: 42, p: [/kontokl[äa]rung.*rente|rente.*kontokl[äa]rung|kontoklärungsbogen.*rente|rente.*konto.*kl[äa]ren/] },
    { key: 'renten_unterlagen', w: 44, p: [/unterlagenanforderung.*rente|rente.*unterlagen.*anfordern|unterlagen.*rentenversicherung|renten.*nachweise.*einreichen/] },
    { key: 'renten_uebertragung', w: 42, p: [/[üu]bertragung.*rente|rente.*[üu]bertragen|renten.*[üu]bertragung|renten.*vertrag.*[üu]bertragen/] },
    { key: 'auslandsrente', w: 44, p: [/auslandsrente|rente.*ausland|auslandsbezug.*rente|rente.*im ausland/] },
    { key: 'beschaeftigungszeiten', w: 42, p: [/besch[aä]ftigungszeiten|nachweis.*besch[aä]ftigungszeiten|rente.*besch[aä]ftigungszeiten|arbeitszeit.*rente.*nachweis/] },
    { key: 'erwerbsminderung', w: 44, p: [/erwerbsminderung|erwerbsminderungsrente|erwerbsminderungs.*rente|berufsunf[aä]hig.*rente/] },
    { key: 'nachversicherung', w: 42, p: [/nachversicherung.*rente|rente.*nachversicherung|nachversicherung.*rentenversicherung|renten.*nachversichern/] },
    { key: 'renten_auszahlungsmitteilung', w: 44, p: [/auszahlungsmitteilung.*rente|rente.*auszahlungsmitteilung|auszahlung.*rente.*mitteilung|rentenzahlung.*mitteilung/] },
    { key: 'versorgungsausgleich', w: 44, p: [/versorgungsausgleich|versorgungsausgleich.*rente|rente.*versorgungsausgleich|renten.*splitting/] },
    { key: 'hinterbliebenenrente', w: 44, p: [/hinterbliebenenrente|hinterbliebenen.*rente|witwenrente|witwerrente|waise.*rente/] },
    { key: 'ueberfuehrungspruefung', w: 44, p: [/[üu]berf[üu]hrungspr[üu]fung|pr[üu]fung.*[üu]berf[üu]hrung|rente.*[üu]berf[üu]hrungspr[üu]fung/] },
    { key: 'kontostandsmeldung', w: 44, p: [/kontostandsmeldung.*rente|rente.*kontostandsmeldung|kontostand.*rente.*melden|rentenkonto.*meldung/] },
    { key: 'renten_abschlag', w: 44, p: [/rentenabschlag|abschlag.*rente|rente.*abschlag|abschlagsfaktor.*rente/] },
    { key: 'altersgrenze', w: 44, p: [/altersgrenze|altersgrenze.*rente|rente.*altersgrenze|regelaltersgrenze/] },
    { key: 'renten_steuerbescheid', w: 44, p: [/rente.*steuerbescheid|steuerbescheid.*rente|renten.*steuer.*bescheid|steuer.*rente.*bescheid/] },
    { key: 'renten_volksversicherung', w: 40, p: [/volksversicherung.*rente|rentenversicherung.*volks|volksversicherung.*rentenanspruch|sozialversicherung.*volksversicherung/] },
    { key: 'rentenbescheid_nachzahlung', w: 48, p: [/nachzahlung.*rente|renten.*nachzahlung|rentenbescheid.*nachzahlung|nachzahlungsbetrag.*rente/] },
    { key: 'renten_widerspruchsbescheid', w: 48, p: [/renten.*widerspruchsbescheid|widerspruchsbescheid.*rente|rente.*widerspruch.*bescheid/] },
    { key: 'rentenueberzahlung', w: 48, p: [/[üu]berzahlung.*rente|rente.*[üu]berzahlt|renten.*r[üu]ckforderung|zu viel gezahlte rente/] },
    { key: 'krankenkasse', w: 38, p: [/krankenkasse|mitgliedsbescheinigung|krankenversicherung|versichertennummer|aok|barmer|techniker krankenkasse|\bdak\b|ikk|kkh/] },
    { key: 'mitgliedschaftsbeginn', w: 44, p: [/mitgliedschaftsbeginn|beginn.*mitgliedschaft|mitgliedschaft.*beginnt|krankenkasse.*beginn.*mitgliedschaft/] },
    { key: 'gesundheitspraemie', w: 44, p: [/gesundheitspr[aä]mie|pr[aä]mie.*krankenkasse|krankenkasse.*pr[aä]mie|pr[aä]mienerstattung.*kranken/] },
    { key: 'beauftragtenvollmacht', w: 44, p: [/beauftragtenvollmacht|vollmacht.*krankenkasse|krankenkasse.*vollmacht|vollmacht.*beauftragten/] },
    { key: 'freistellungsbescheinigung', w: 44, p: [/freistellungsbescheinigung|freistellung.*beitrag.*kranken|krankenkasse.*freistellung|freistellungsbescheinigung.*steuer/] },
    { key: 'beitragsausgleich', w: 44, p: [/beitragsausgleich|beitrags.*ausgleich.*kranken|krankenkasse.*beitragsausgleich|ausgleich.*beitr[äa]ge/] },
    { key: 'weiterbildungszuschuss', w: 44, p: [/weiterbildungszuschuss.*kranken|krankenkasse.*weiterbildung|zuschuss.*weiterbildung.*kranken|weiterbildungs.*zuschuss/] },
    { key: 'praevention', w: 44, p: [/pr[aä]vention|pr[äa]ventiv.*krankenkasse|präventiv.*kasse/] },
    { key: 'krankenkassen_ablehnung', w: 44, p: [/kranken.*(antrag|leistung).*abgelehnt|ablehnung.*krankenkasse|krankenkassenablehnungsbescheid|ablehnungsbescheid.*kranken/] },
    { key: 'krankenversicherungsverlauf', w: 44, p: [/krankenversicherungsverlauf|verlauf.*krankenversicherung|kranken.*versicherungsverlauf|beitragsverlauf.*kranken/] },
    { key: 'sonderzahlung', w: 42, p: [/sonderzahlung.*kranken|krankenkasse.*sonderzahlung|sonderzahlung.*krankenkasse/] },
    { key: 'fragebogen_auswertung', w: 44, p: [/fragebogen.*auswertung|auswertung.*fragebogen|fragebogen.*krankenkasse.*ausgewertet|mitgliedschaftsfragebogen.*auswertung/] },
    { key: 'beitragsaenderung', w: 44, p: [/beitrags[aä]nderung.*kranken|kranken.*beitrag.*[aä]ndern|beitrag.*wird.*ge[äa]ndert|neuer.*beitrag.*krankenkasse/] },
    { key: 'krankengeld', w: 46, p: [/krankengeld|krankengeld.*krankenkasse|krankenkasse.*krankengeld|krankengeldanspruch/] },
    { key: 'krankenkasse_aenderung', w: 44, p: [/krankenkassen[aä]nderung|[äa]nderungsmitteilung.*kranken|mitgliedschafts[aä]nderung|personalien[aä]nderung.*krankenkasse/] },
    { key: 'gesundheitskur', w: 44, p: [/gesundheitskur|vorsorgekur|rehabilitationsma[äa]hnahme.*kasse|krankenkasse.*(kur|reha)|rehabilitationsma[äa]hnahme/] },
    { key: 'mitgliedsbescheinigung_beitraege', w: 46, p: [/mitgliedsbescheinigung.*beitr[äa]ge|beitr[äa]ge.*mitgliedsbescheinigung|mitgliedsnachweis.*beitrag|beitrag.*mitgliedsnachweis/] },
    { key: 'krankenkasse_einkommen', w: 44, p: [/einkommensnachweis.*kranken|krankenkasse.*einkommensnachweis|fragebogen.*familienversicherung|familienversicherung.*fragebogen/] },
    { key: 'krankenkassen_wechsel', w: 42, p: [/krankenkassenwechsel|kassenwechsel|mitgliedschaft.*wechsel|kassenwechsel.*kranken/] },
    { key: 'pflegekasse_bescheid', w: 46, p: [/pflegekasse.*bescheid|bescheid.*pflegegrad|pflegegrad.*bescheid|pflegegrad.*festgestellt/] },
    { key: 'zahnersatz', w: 46, p: [/zahnersatz|kostenvoranschlag.*zahn|hei[l]mittel|krankenkasse.*zahnarzt|zahnarzt.*krankenkasse|kieferorthop[äa]die/] },
    { key: 'zahnersatz_erstattung', w: 48, p: [/zahnersatz.*erstattung|erstattung.*zahnersatz|kosten.*erstattung.*zahn|zahnersatz.*kasse.*erstattet/] },
    { key: 'krankenkassen_zahnersatz', w: 48, p: [/zahnersatz.*bescheid|bescheid.*zahnersatz|zahnersatz.*krankenkasse.*(genehmigt|abgelehnt)|krankenkasse.*zahnersatz.*bescheid/] },
    { key: 'krankenkassen_bescheid', w: 44, p: [/beitragsbescheid.*krankenkasse|beitragsnachweis.*krankenversicherung|krankenkasse.*zahlungsaufforderung|selbsteinkommen|krankenkassenbescheid|beitr[äa]ge.*krankenkasse/] },
    { key: 'mitgliedsbeitrag_rueck', w: 44, p: [/beitragsr[üu]ckerstattung|mitgliedsbeitrag.*erstattet|beitrags.*r[üu]ckerstattung.*kranken|beitrag.*gutschrift.*krankenkasse/] },
    { key: 'beitragsbefreiung', w: 46, p: [/beitragsbefreiung|befreiung.*beitrag.*kranken|krankenkasse.*befreit|befreiung.*krankenversicherung/] },
    { key: 'beitragsbescheid', w: 40, p: [/beitragsbescheid|beitragsnachweis|beitragsrechnung|beitragsfestsetzung|mitgliedsbeitr[äa]ge/] },
    { key: 'arbeitsunfaehigkeit', w: 38, p: [/arbeitsunf[aä]hig|krankschreibung|erstbescheinigung|folgebescheinigung|[äa]rztliches attest|\beau\b|eau\b/] },

    // Tax
    { key: 'steuerbescheid', w: 40, p: [/einkommensteuerbescheid|festgesetzte steuer|steuerguthaben|steuernachzahlung|steuerbescheid/] },
    { key: 'steuer_bescheidankuendigung', w: 42, p: [/ank[üu]ndigung.*steuerbescheid|steuerbescheid.*angek[üu]ndigt|vorank[üu]ndigung.*bescheid|steuer.*wird.*bescheid/] },
    { key: 'steuerbescheid_widerruf', w: 48, p: [/widerruf.*steuerbescheid|steuerbescheid.*widerruf|widerruf.*steuerfestsetzung/] },
    { key: 'steuer_korrektur', w: 46, p: [/steuerbescheid.*(korrektur|[äa]nderung)|korrektur.*steuerbescheid|[äa]nderungsbescheid.*steuer|steuerbescheid.*ge[äa]ndert/] },
    { key: 'steuerbescheid_einspruch', w: 48, p: [/einspruch.*(einkommensteuer|steuerbescheid)|steuerbescheid.*einspruch|einspruchsfrist.*steuer/] },
    { key: 'steuer_erstattung', w: 36, p: [/steuererstattung|erstattungsbetrag|steuer.*erstattet|gutschrift.*finanzamt/] },
    { key: 'steuer_nachzahlung', w: 46, p: [/nachzahlung.*steuer|steuernachzahlung|zu zahlende steuer|steuer.*nachzahlen|saumniszuschlag.*steuer/] },
    { key: 'steuer_nebenrechnung', w: 40, p: [/nebenrechnung.*steuer|steuer.*nebenrechnung|nebenrechnung.*finanzamt|steuerliche nebenrechnung/] },
    { key: 'steuer_vorauszahlung', w: 44, p: [/vorauszahlung.*steuer|steuer.*vorauszahlung|vorauszahlungsbescheid|steuer.*vorauszahlungsbetrag/] },
    { key: 'steuer_bescheiderklaerung', w: 48, p: [/\bbescheiderkl[äa]rung\b|erkl[äa]rung.*steuerbescheid/] },
    { key: 'steuer_erklaerungsassistent', w: 42, p: [/erklaerungsassistent|steuer.*assistent|assistent.*steuererkl[äa]rung|hilfe.*steuererkl[äa]rung/] },
    { key: 'steuer_erlass', w: 46, p: [/steuer.*erlass|erlass.*steuer|erlassbescheid.*steuer|steuer.*erlassen/] },
    { key: 'steuer_vertretung', w: 42, p: [/steuer.*vertretung|vertretung.*steuer|vollmacht.*steuer|steuerberater.*vertretung/] },
    { key: 'sammelbescheid', w: 44, p: [/sammelbescheid.*steuer|steuer.*sammelbescheid|sammelbescheid.*finanzamt/] },
    { key: 'behoerdenauskunft', w: 44, p: [/beh[oö]rdenauskunft.*steuer|steuer.*beh[oö]rdenauskunft|auskunft.*finanzamt.*beh[oö]rde|finanzamt.*beh[oö]rdenauskunft/] },
    { key: 'steuer_dauerzahlung', w: 44, p: [/dauerzahlung.*steuer|steuer.*dauerzahlung|steuer.*dauerzahler|dauerzahlung.*finanzamt/] },
    { key: 'lohnsteuerbescheinigung', w: 46, p: [/lohnsteuerbescheinigung|lohnsteuerbesch.*bescheinigung|elektronische.*lohnsteuer|bescheinigung.*lohnsteuer/] },
    { key: 'steuer_freibetrag', w: 44, p: [/freibetrag.*steuer|steuer.*freibetrag|freibetragsbescheid|kinderfreibetrag.*steuer/] },
    { key: 'einkunftsnachweis', w: 44, p: [/einkunftsnachweis|nachweis.*eink[üu]nfte|einkünfte.*nachweisen|einkunftsart.*nachweis/] },
    { key: 'umsatzsteuer_voranmeldung', w: 44, p: [/umsatzsteuer.*voranmeldung|voranmeldung.*umsatzsteuer|ust.*voranmeldung|umsatzsteuervoranmeldung/] },
    { key: 'spendenbescheinigung', w: 44, p: [/spendenbescheinigung|spende.*bescheinigung.*steuer|zuweisung.*spende.*steuer|bescheinigung.*spende/] },
    { key: 'gewinnermittlung', w: 44, p: [/gewinnermittlung|ermittlung.*gewinn|gewinn.*ermittelt|ueberschussrechnung|einnahmen.*[üu]berschuss.*rechnung/] },
    { key: 'sonderausgaben', w: 44, p: [/sonderausgaben|sonderausgabe.*steuer|sonderausgaben.*abzug|vorsorgeaufwendungen.*sonderausgaben/] },
    { key: 'verlustabzug', w: 44, p: [/verlustabzug|abzug.*verlust|verlust.*abzug.*steuer|verlustvortrag/] },
    { key: 'erklaerungspflicht', w: 44, p: [/erkl[aä]rungspflicht|pflicht.*steuererkl[aä]rung|abgabepflicht.*steuer|steuererkl[aä]rung.*pflicht/] },
    { key: 'finanzamt_erinnerung', w: 36, p: [/abgabe der steuererkl[äa]rung|steuererkl[äa]rung.*sp[äa]testens|erinnerung.*steuererkl/] },
    { key: 'tax_id', w: 40, p: [/steueridentifikationsnummer|steuer[- ]?id|bundeszentralamt f[iu]r steuern|\bbzst\b|\bidnr\b|identifikationsnummer/] },
    { key: 'elster', w: 32, p: [/elster|elektronische steuererkl[äa]rung|zugangsdaten.*finanzamt/] },
    { key: 'steuer_datenuebersicht', w: 36, p: [/daten[üu]bersicht.*steuer|steuer.*daten[üu]bersicht|meine daten.*finanzamt|steuerkonto.*[üu]bersicht/] },

    // Jobcenter / social
    { key: 'job', w: 40, p: [/aufforderung zur mitwirkung|mitwirkungspflicht|jobcenter.*mitwirkung/] },
    { key: 'arbeitslosengeld', w: 34, p: [/arbeitslosengeld|alg ?1|leistungsbescheid.*arbeitsagentur|arbeitslosengeldbescheid/] },
    { key: 'selbststaendigkeit', w: 44, p: [/selbstst[aä]ndigkeit.*alg|alg.*selbstst[aä]ndigkeit|selbstst[aä]ndiger.*alg|selbstst[aä]ndig.*arbeitslosengeld/] },
    { key: 'alg_bescheid_korrektur', w: 44, p: [/alg.*bescheid.*korrektur|korrektur.*alg.*bescheid|alg.*bescheid.*ge[äa]ndert|arbeitslosengeld.*bescheid.*korrigiert/] },
    { key: 'alg_nachzahlungsmitteilung', w: 44, p: [/nachzahlungsmitteilung.*alg|alg.*nachzahlungsmitteilung|nachzahlung.*arbeitslosengeld.*mitteilung|alg.*nachzahlung.*bescheid/] },
    { key: 'nebeneinkommen', w: 44, p: [/nebeneinkommen.*alg|alg.*nebeneinkommen|meldung.*nebeneinkommen|nebenverdienst.*melden/] },
    { key: 'meldepflicht', w: 42, p: [/meldepflicht.*arbeitslos|meldepflicht.*alg|alg.*meldepflicht|arbeitssuche.*melden|meldung.*arbeitsagentur/] },
    { key: 'alg_weiterbewilligungsantrag', w: 68, p: [/weiterbewilligungsantrag.*(alg|arbeitslosengeld)|(alg|arbeitslosengeld).*weiterbewilligungsantrag|antrag.*weiterbewilligung.*arbeitslosengeld|fortzahlungsantrag.*alg/] },
    { key: 'arbeitsbescheinigung', w: 40, p: [/arbeitsbescheinigung|bescheinigung.*arbeitgeber.*alg|arbeitgeberbescheinigung|arbeitszeitbescheinigung/] },
    { key: 'alg_neufestsetzung', w: 46, p: [/neufestsetzung.*arbeitslosengeld|alg.*neufestsetzung|arbeitslosengeld.*neu festgesetzt|neufestsetzungsbescheid.*alg/] },
    { key: 'alg_erstantrag', w: 42, p: [/erstantrag.*arbeitslosengeld|arbeitslosengeld.*erstantrag|erstmaligen.*alg.*stellen|erstantrag.*alg/] },
    { key: 'alg_bemessung', w: 42, p: [/bemessung.*arbeitslosengeld|alg.*bemessungsbescheid|bemessungsbescheid.*alg|bemessungsgrundlage.*alg/] },
    { key: 'alg_antrag', w: 38, p: [/antrag.*arbeitslosengeld|arbeitslosengeld.*antrag|alg.*antrag.*stellen|antragsformular.*alg/] },
    { key: 'alg_sperrzeit', w: 44, p: [/sperrzeit.*arbeitslosengeld|alg.*sperrzeit|sperrzeit.*alg|sanktion.*alg/] },
    { key: 'alg_weiterbewilligung', w: 64, p: [/weiterbewilligung.*arbeitslosengeld|arbeitslosengeld.*weiterbewilligt|alg.*weiterbewilligungsbescheid|fortzahlung.*alg/] },
    { key: 'alg_ablehnung', w: 44, p: [/ablehnung.*arbeitslosengeld|arbeitslosengeld.*abgelehnt|alg.*abgelehnt|ablehnungsbescheid.*alg/] },
    { key: 'alg_bewilligung', w: 64, p: [/\b(arbeitslosengeld|alg)[-_ ]+bewilligungsbescheid\b|bewilligungsbescheid.*(arbeitslosengeld|alg)/] },
    { key: 'alg_widerspruch', w: 50, p: [/widerspruch.*arbeitslosengeld|arbeitslosengeld.*widerspruch|alg.*widerspruchsbescheid|widerspruchsbescheid.*alg/] },
    { key: 'kindergeld', w: 34, p: [/familienkasse|kindergeld|kinderzuschlag/] },
    { key: 'elterngeld', w: 38, p: [/elterngeldaktenzeichen|elterngeld|elterngeldbescheid|basiselterngeld|elterngeldplus/] },
    { key: 'elterngeld_widerruf', w: 46, p: [/widerruf.*elterngeld|elterngeld.*widerruf|elterngeld.*aufhebung|r[üu]ckforderung.*elterngeld/] },
    { key: 'wohngeld', w: 36, p: [/wohngeldbescheid|wohngeld|lastenbeihilfe/] },
    { key: 'mietzuschuss', w: 40, p: [/mietzuschuss|zuschuss.*miete|wohnkostenzuschuss|mietbeihilfe/] },
    { key: 'bafoeg', w: 36, p: [/baf[oö]g|ausbildungsf[oö]rderung|f[oö]rderungsbescheid/] },
    { key: 'bafoeg_zulage', w: 42, p: [/baf[oö]g.*zulage|zulage.*baf[oö]g|auslandszuschlag.*baf[oö]g|bedarfssatz.*baf[oö]g/] },
    { key: 'hoechstdauer', w: 44, p: [/h[oö]chstdauer.*baf[oö]g|baf[oö]g.*h[oö]chstdauer|h[oö]chstdauer.*f[oö]rderung|f[oö]rderung.*maximaldauer/] },
    { key: 'foerderungsende', w: 44, p: [/\bf[oö]rderungsende\b|baf[oö]g.*(endet|beendet)|f[oö]rderung.*(endet|beendet)|ende der f[oö]rderung/] },
    { key: 'einkommensanrechnung', w: 44, p: [/einkommensanrechnung.*baf[oö]g|baf[oö]g.*einkommensanrechnung|anrechnung.*einkommen.*f[oö]rderung/] },
    { key: 'bafoeg_aenderungsbescheid', w: 44, p: [/\bbaf[oö]g[aä]nderungsbescheid\b|baf[oö]g.*[äa]nderungsbescheid|[äa]nderungsbescheid.*baf[oö]g|baf[oö]g.*ge[äa]ndert/] },
    { key: 'bafoeg_erstausstattung', w: 44, p: [/erstausstattung.*baf[oö]g|baf[oö]g.*erstausstattung|erstausstattungsbedarf/] },
    { key: 'bafoeg_verlaengerung', w: 44, p: [/verl[aä]ngerung.*baf[oö]g|baf[oö]g.*verl[aä]ngerung|weiterf[oö]rderung.*baf[oö]g|f[oö]rderung.*verl[aä]ngert/] },
    { key: 'foerderungsverzicht', w: 44, p: [/verzicht.*baf[oö]g|baf[oö]g.*verzicht|verzicht.*f[oö]rderung|foerderungsverzicht/] },
    { key: 'bafoeg_nachweis_einkommen', w: 44, p: [/nachweis.*einkommen.*baf[oö]g|baf[oö]g.*einkommensnachweis|einkommensnachweis.*ausbildungsf[oö]rderung|eltern.*einkommensnachweis.*baf[oö]g/] },
    { key: 'zahlungsanweisung', w: 42, p: [/zahlungsanweisung.*baf[oö]g|baf[oö]g.*zahlungsanweisung|auszahlungsanweisung.*f[oö]rderung/] },
    { key: 'bafoeg_widerspruch', w: 46, p: [/widerspruch.*baf[oö]g|baf[oö]g.*widerspruch|widerspruch.*ausbildungsf[oö]rderung/] },
    { key: 'bafoeg_rueckzahlung', w: 44, p: [/r[üu]ckzahlung.*baf[oö]g|baf[oö]g.*r[üu]ckzahlung|r[üu]ckforderung.*baf[oö]g|teil.*r[üu]ckzahlung.*baf[oö]g/] },
    { key: 'bafoeg_erstantrag', w: 40, p: [/erstantrag.*baf[oö]g|baf[oö]g.*erstantrag|erstmalige.*baf[oö]g|erstantrag.*ausbildungsf[oö]rderung/] },    { key: 'bafoeg_vorleistung', w: 42, p: [/vorleistung.*baf[oö]g|baf[oö]g.*vorleistung|vorleistungsbescheid.*baf[oö]g/] },
    { key: 'bafoeg_ablehnung', w: 44, p: [/ablehnung.*baf[oö]g|baf[oö]g.*abgelehnt|ablehnungsbescheid.*baf[oö]g|f[oö]rderung.*abgelehnt/] },
    { key: 'bafoeg_elternunabhaengig', w: 42, p: [/elternunabh[äa]ngig.*baf[oö]g|baf[oö]g.*elternunabh[äa]ngig|elternunabh[äa]ngige.*f[oö]rderung/] },
    { key: 'bafoeg_widerruf', w: 44, p: [/widerruf.*baf[oö]g|baf[oö]g.*widerruf|widerruf.*ausbildungsf[oö]rderung/] },

    // Alien / migration
    { key: 'auslaender', w: 38, p: [/ausl[äa]nderbeh[öo]rde|landesamt f[üu]r einwanderung|\blea\b|aufenthaltstitel|aufenthaltserlaubnis|niederlassungserlaubnis|duldung/] },
    { key: 'einbuergerung', w: 36, p: [/einb[üu]rgerung|einb[üu]rgerungsantrag|staatsangeh[öo]rigkeit.*erwerb|einb[üu]rgerungsbescheid/] },

    // Court / fines
    { key: 'mahnbescheid', w: 42, p: [/gerichtlicher mahnbescheid|mahngericht|amtsgericht.*mahnbescheid|vollstreckungsbescheid|zwangsvollstreckung/] },
    { key: 'vollstreckung', w: 36, p: [/vollstreckungsank[iu]ndigung|pf[äa]ndung|konto[pfü]ndung|gerichtsvollzieher/] },
    { key: 'pfaendungsschutz', w: 40, p: [/pf[äa]ndungsschutz|schutzkonto|pfandkonto|pf[äa]ndung vorbeugend|unpf[aä]ndbar/] },
    { key: 'arbeitsgericht_klage', w: 38, p: [/arbeitsgericht|k[üu]ndigungsschutzklage|fristlose k[üu]ndigung.*klage|klage.*arbeitsverh[äa]ltnis|richter.*g[üu]teverhandlung/] },
    { key: 'widerspruch', w: 46, p: [/widerspruch gegen (den|denen|einen)|einspruch gegen|rechtsbehelf gegen|widerspruch.*(eingereicht|erhoben|bescheid)/] },
    { key: 'widerspruchsbescheid', w: 48, p: [/\bwiderspruchsbescheid\b|widerspruch.*zur[üu]ckgewiesen|widerspruch.*stattgegeben/] },
    { key: 'bussgeldbescheid', w: 38, p: [/bu[ss]geldbescheid|anh[oö]rungsbogen|zeugenfragebogen|ordnungswidrigkeit|geldbu[ss]e/] },
    { key: 'kfz_steuer', w: 44, p: [/kraftfahrzeugsteuer|kfz[- ]?steuer|fahrzeugsteuer/] },
    { key: 'kfz_steuer_neu', w: 46, p: [/kfz.*steuer.*neu|neufestsetzung.*kfz|kfz.*steuer.*[äa]nderung|fahrzeugsteuer.*neu/] },
    { key: 'kfz_steuer_zahlung', w: 46, p: [/kfz.*steuer.*zahlung|kfz.*steuer.*zahlen|kfz-steuerbescheid.*zahlung|fahrzeugsteuer.*f[aä]llig/] },
    { key: 'kfz_steuer_rueckerstattung', w: 46, p: [/kfz.*steuer.*r[üu]ckerstattung|r[üu]ckerstattung.*kfz.*steuer|fahrzeugsteuer.*erstattet|kfz.*steuer.*erstattet/] },
    { key: 'kfz_steuer_ummeldung', w: 46, p: [/kfz.*steuer.*ummeldung|ummeldung.*kfz.*steuer|fahrzeug.*umgemeldet.*steuer|kfz.*steuer.*[üu]meldung/] },
    { key: 'kfz_steuer_aussetzung', w: 46, p: [/kfz.*steuer.*aussetzung|aussetzung.*kfz.*steuer|kfz.*steuer.*ausgesetzt|fahrzeugsteuer.*aussetzung/] },
    { key: 'halterwechsel', w: 46, p: [/halterwechsel|halter.*wechsel.*kfz|kfz.*halter.*gewechselt|fahrzeughalter.*wechsel/] },
    { key: 'abmahnung', w: 34, p: [/abmahnung|arbeitsrechtlich.*abmahnung|formalabmahnung/] },

    // Rundfunk
    { key: 'gez', w: 38, p: [/rundfunkbeitrag|beitragsservice|ard zdf deutschlandradio|geb[iu]hreneinzugszentrale/] },
    { key: 'gez_nachzahlung', w: 44, p: [/nachzahlung.*rundfunkbeitrag|rundfunkbeitrag.*nachzahlung|s[äa]umniszuschlag|ausstehende.*beitr[äa]ge/] },
    { key: 'gez_nachholzahlung', w: 44, p: [/nachholzahlung.*rundfunk|rundfunk.*nachholzahlung|nacherhebung.*beitrag|beitrag.*nacherhoben/] },
    { key: 'gez_teilerstattung', w: 44, p: [/erstattung.*rundfunkbeitrag|rundfunkbeitrag.*erstattet|teil.*erstattung.*rundfunk|r[üu]ckerstattung.*beitrag/] },
    { key: 'gez_zahlungsplan', w: 42, p: [/zahlungsplan.*rundfunk|rundfunk.*zahlungsplan|raten.*rundfunkbeitrag|stundung.*beitrag/] },
    { key: 'gez_kontoauszug', w: 44, p: [/kontoauszug.*rundfunkbeitrag|rundfunkbeitrag.*kontoauszug|zahlungs[üu]bersicht.*rundfunk|beitragskonto/] },
    { key: 'gez_befreiung', w: 44, p: [/befreiung.*rundfunkbeitrag|rundfunkbeitrag.*befreiung|befreiungsantrag/] },
    { key: 'gez_wohnungswechsel', w: 44, p: [/rundfunkbeitrag.*wohnungswechsel|wohnungswechsel.*rundfunk|gez.*umzug|umzug.*beitragsservice/] },
    { key: 'gez_meldedaten', w: 44, p: [/meldedatenabgleich.*gez|gez.*meldedaten|meldedaten.*beitragsservice|datenabgleich.*rundfunk/] },
    { key: 'gez_abmeldung', w: 46, p: [/abmeldung.*rundfunkbeitrag|rundfunkbeitrag.*abmelden|abmeldung.*beitragsservice|gez.*abmeldung/] },
    { key: 'gez_zahlungsverbot', w: 46, p: [/zahlungsverbot.*rundfunk|rundfunk.*zahlungsverbot|verf[üu]gung.*beitragszahlung|einstellung.*beitragszahlung/] },
    { key: 'gez_befreiungs_ablehnung', w: 46, p: [/befreiung.*abgelehnt|befreiungsantrag.*abgelehnt|befreiung.*abgewiesen|befreiung.*nicht.*ge[äa]ndert/] },

    // Registration / residence
    { key: 'melde', w: 36, p: [/meldebest[äa]tigung|wohnsitzanmeldung|anmeldung bei der meldebeh[öo]rde/] },
    { key: 'gewerbeanzeige', w: 44, p: [/gewerbeanzeige|gewerbeanmeldung|gewerbeurkunde|gewerbe[äa]nderung|gewerbeerlaubnis/] },
    { key: 'integrationskurs', w: 44, p: [/integrationskurs|integrationskursus|sprachkurs.*integration|berufssprachkurs/] },
    { key: 'gebuehrenbescheid', w: 38, p: [/geb[üu]hrenbescheid|geb[üu]hrenrechnung|verwaltungsgeb[üu]hr|amtliche geb[üu]hr/] },
    { key: 'umzugsmitteilung', w: 30, p: [/umzugsmitteilung|wohnungswechsel|abmeldebescheinigung|adressanderung/] },

    // Insurance (private & protection, distinct from health)
    { key: 'kfz_versicherung', w: 40, p: [/kfz[- ]?versicherung|fahrzeugversicherung|kaskoversicherung|kfz.*haftpflicht|fahrzeug.*haftpflicht|versicherungsbest[äa]tigung.*kfz/] },
    { key: 'kfz_bonus_malus', w: 44, p: [/bonus[- ]?malus|bonus-malus.*kfz|kfz.*bonus.*malus|malus.*stufe/] },
    { key: 'kfz_abrechnung', w: 44, p: [/kfz.*versicherung.*abrechnung|abrechnung.*kfz.*versicherung|kfz.*beitragsabrechnung|versicherungsabrechnung.*kfz/] },
    { key: 'versicherungsschutzbestaetigung', w: 46, p: [/versicherungsschutzbest[aä]tigung|schutzbestaetigung.*versicherung|doppel.*versicherungsschutz|versicherungsschutz.*best[aä]tigung/] },
    { key: 'kfz_vollkasko', w: 44, p: [/\bvollkasko\b|vollkaskoversicherung|teil.*kasko.*vollkasko|kfz.*vollkasko/] },
    { key: 'kfz_versicherungserhoehung', w: 44, p: [/kfz.*beitrag.*erh[oö]hung|beitrag.*erh[oö]hung.*kfz|kfz.*versicherung.*teurer|kfz.*beitrag.*[aä]nderung/] },
    { key: 'kfz_haftpflicht_erhoeht', w: 46, p: [/haftpflicht.*erh[oö]hung|kfz.*haftpflicht.*erh[oö]ht|haftpflichtbeitrag.*erh[oö]ht/] },
    { key: 'kfz_wechsel', w: 42, p: [/kfz.*versicherung.*wechsel|versicherungswechsel.*kfz|kfz.*police.*wechsel|kfz.*versicherer.*gewechselt/] },
    { key: 'haftpflicht_versicherung', w: 34, p: [/haftpflichtversicherung|privat[- ]?haftpflicht|betriebshaftpflicht/] },
    { key: 'hausrat_versicherung', w: 34, p: [/hausratversicherung|hausratversicherungsschutz/] },
    { key: 'versicherungsschein', w: 30, p: [/versicherungsschein|versicherungspolice|police.*versicherung/] },
    { key: 'beitragserhoehung', w: 32, p: [/beitragserh[oö]hung|beitrags.[aä]nderung|pr[äa]mium.*erh[oö]hung|tariferh[oö]hung/] },
    { key: 'bonusprogramm', w: 46, p: [/bonusprogramm|bonusheft|gesundheitskarte.*bonus|pr[äa]mienprogramm/] },
    { key: 'bonusscheck', w: 48, p: [/bonusscheck|bonus.*gutschrift.*kranken|bonus.*auszahlung.*kranken|bonus.*erstattet.*krankenkasse/] },
    { key: 'versicherung_schaden', w: 36, p: [/schadenmeldung|schadensfall|versicherung.*schaden|regulierung|schadensnummer/] },
    { key: 'kfz_schadenregulierung', w: 44, p: [/kfz.*schaden.*regulierung|schadenregulierung.*kfz|unfallbericht.*versicherung|fahrzeug.*schaden.*melden/] },
    { key: 'kfz_schaden_ablehnung', w: 46, p: [/kfz.*schaden.*abgelehnt|abgelehnt.*kfz.*schaden|schaden.*abgelehnt.*kfz|kfz.*schaden.*keine.*zahlung/] },
    { key: 'kfz_kostenuebernahme', w: 46, p: [/kfz.*kosten[üu]bernahme|kosten[üu]bernahme.*kfz|kfz.*schaden.*kosten.*[üu]bern|kfz.*kosten.*[üu]bernommen/] },
    { key: 'kasko_schaden', w: 46, p: [/\bkaskoschaden\b|kasko.*schadensfall|schadensfall.*kasko|kasko.*schadensmeldung/] },
    { key: 'versicherungswiderspruch', w: 46, p: [/widerspruch.*versicherung|versicherung.*widerspruch|widerspruch.*versicherungsschein|leistungsfall.*widerspruch/] },
    { key: 'bausparvertrag', w: 34, p: [/bausparvertrag|bausparkasse|bausparsumme/] },
    { key: 'pflegeversicherung', w: 32, p: [/pflegeversicherung|pflegekasse|pflegegrad|pflegebed[iu]rftig/] },
    { key: 'versicherungsverlauf', w: 32, p: [/versicherungsverlauf|versicherungszeiten|beitragsverlauf|sozialversicherungsverlauf/] },
    { key: 'lebensversicherung', w: 32, p: [/lebensversicherung|kapitallebensversicherung|risikoversicherung|private rentenversicherung/] },
    { key: 'kuendigung', w: 34, p: [/k[üu]ndigung|k[üu]ndigungsschutz|k[üu]ndigungsfrist|beendigung des/] },
    { key: 'sonderkuendigungsrecht', w: 44, p: [/sonderk[üu]ndigung|sonderk[üu]ndigungsrecht|au[ßs]erordentliche k[üu]ndigung|k[üu]ndigung.*sonder/] },

    // Banking & finance
    { key: 'kontoauszug', w: 40, p: [/kontoauszug|kontoauszug.*konto|umsatzaufstellung|wertstellung|kontostand/] },
    { key: 'kontoverbindung', w: 40, p: [/kontoverbindung|verbindung.*konto|kontoverbindung.*bank|bank.*kontoverbindung/] },
    { key: 'pin_brief', w: 50, p: [/pin[- ]?brief.*(bank|sparkasse|volksbank|girocard|kreditkarte)|(bank|sparkasse).*pin[- ]?brief|pin[- ]?mitteilung.*(bank|sparkasse|konto)/] },
    { key: 'girocard', w: 40, p: [/girocard|girokarte|girocard.*konto|bankkarte.*girocard/] },
    { key: 'umbuchung', w: 40, p: [/umbuchung|umbuchung.*konto|konto.*umgebucht|[üu]bertrag.*[üu]mbuchung/] },
    { key: 'zinsabrechnung', w: 40, p: [/zinsabrechnung|zinsrechnung.*konto|zinsen.*abgerechnet|habenzins.*abrechnung/] },
    { key: 'einzahlungsbeleg', w: 40, p: [/einzahlungsbeleg|beleg.*einzahlung|einzahlung.*beleg|einzahlungs.*quittung/] },
    { key: 'waehrungsrechnung', w: 40, p: [/w[aä]hrungsrechnung|rechnung.*w[aä]hrung|w[aä]hrungsumrechnung|fremdw[aä]hrung.*konto/] },
    { key: 'auszahlungskarte', w: 40, p: [/auszahlungskarte|karte.*auszahlung|auszahlung.*karte|karte.*geld.*abheben/] },
    { key: 'ueberweisungsauftrag', w: 40, p: [/[üu]berweisungsauftrag|zahlungsauftrag|[üu]berweisungsaufforderung/] },
    { key: 'widerruf_unterlagen', w: 40, p: [/widerruf.*unterlagen|unterlagen.*widerruf|widerrufs.*unterlagen.*konto|unterlagen.*widerrufen/] },
    { key: 'zahlungsverkehr', w: 38, p: [/zahlungsverkehr|zahlungsverkehrs[üu]bersicht|konto.*zahlungen.*[üu]bersicht|zahlungsums[äa]tze/] },
    { key: 'konto_wechselbank', w: 40, p: [/kontowechsel|wechsel.*bank|bank.*gewechselt|konto.*neue bank|kontowechselservice/] },
    { key: 'kartenwechsel', w: 48, p: [/kartenwechsel|bankkarte.*neu|neue bankkarte|karten.*ersatz|karte.*gewechselt/] },
    { key: 'kartensperre', w: 46, p: [/kartensperre|girocard.*gesperrt|kreditkarte.*gesperrt|bankkarte.*gesperrt|karte.*gesperrt.*konto|sperrung.*(bankkarte|girokarte|kreditkarte)/] },
    { key: 'kontokartennummer', w: 36, p: [/kartennummer|kreditkartennummer|bankkartennummer|karten.*nummer.*konto/] },
    { key: 'konto_ueberziehung', w: 38, p: [/[üu]berziehung.*konto|konto.*[üu]berzogen|[üu]berziehungszins|dispokredit|kreditlinie.*[üu]berschritten/] },
    { key: 'sepa_mandat', w: 30, p: [/sepa[- ]?lastschrift|lastschriftmandat|mandatsreferenz|gl[aä]ubiger.?id/] },
    { key: 'dauerauftrag', w: 32, p: [/dauerauftrag|dauerauftragsbestaetigung|termin[üu]berweisung|regel[üu]berweisung/] },
    { key: 'dauerauftrag_aenderung', w: 42, p: [/dauerauftrag.*([äa]nderung|angepasst)|[äa]nderung.*dauerauftrag|dauerauftrag.*ge[äa]ndert/] },
    { key: 'dauerauftrag_storno', w: 46, p: [/dauerauftrag.*storno|storno.*dauerauftrag|dauerauftrag.*storniert|dauerauftrag.*gel[öo]scht/] },
    { key: 'iban_nachweis', w: 30, p: [/\bibans\b|schuldner-[iI]ban|zahlungsempf[äa]nger.*iban|iban.*benachrichtigung|sammel[üu]berweisung/] },
    { key: 'bank_gebuehren', w: 34, p: [/kontogeb[iu]hr|grundgeb[iu]hr|preisliste.*konto|entgelt.*konto|kontoentgelt/] },
    { key: 'kredit_vertrag', w: 36, p: [/kreditvertrag|darlehensvertrag|ratenkredit|tilgungsplan|annuit[äa]tendarlehen/] },
    { key: 'sparkonto', w: 30, p: [/sparvertrag|spareinlage|vorsorgevertrag|kapitalanlage/] },
    { key: 'inkasso', w: 44, p: [/inkasso|inkassob[iu]ro|forderungsmanagement|zahlungsaufforderung.*inkasso|beitreibung/] },
    { key: 'schufa_auskunft', w: 38, p: [/schufa|bonit[aä]tsauskunft|kreditw[iu]rdigkeit.*auskunft|selbstauskunft.*schufa/] },
    { key: 'kontoschliessung', w: 34, p: [/kontoschliessung|kontoaufl[oö]sung|konto wird geschlossen/] },
    { key: 'kontokuendigung', w: 38, p: [/\bkontok[üu]ndigung\b|k[üu]ndigung des kontos|konto (wird|wurde|zu) k[üu]ndigen/] },

    // Utility & housing
    { key: 'rechnung', w: 30, p: [/rechnung\b|rechnungsnummer|rechnungsbetrag|zahlbar bis|skonto|verbrauchsabrechnung/] },
    { key: 'mahnung', w: 38, p: [/zahlungserinnerung|1\.\s*mahnung|2\.\s*mahnung|letzte mahnung|mahngeb[iu]hr|inkasso|dunning|zahlungsaufforderung/] },
    { key: 'nebenkosten', w: 36, p: [/betriebskostenabrechnung|nebenkostenabrechnung|abrechnung der betriebskosten|betriebskostenabrechnung/] },
    { key: 'stadtwerke', w: 34, p: [/stadtwerke|stromabrechnung|gasabrechnung|jahresverbrauchsabrechnung|abschlag|energieversorger/] },
    { key: 'grundversorgung', w: 40, p: [/grundversorgung|grundversorger|grundversorgung.*strom|grundversorgung.*energie/] },
    { key: 'netzbetreiber', w: 40, p: [/netzbetreiber|netz.*betreiber|stromnetz.*betreiber|netzbetrieb.*stadtwerke/] },
    { key: 'gasbezug', w: 40, p: [/gasbezug|bezug.*gas|gaslieferung.*beginn|gas.*beliefert/] },
    { key: 'lieferaufnahme', w: 42, p: [/lieferaufnahme|aufnahme.*belieferung|belieferung.*beginn|lieferung.*wird.*aufgenommen/] },
    { key: 'begruessungsschreiben', w: 40, p: [/begr[üu]ssung.*stadtwerke|stadtwerke.*begr[üu]ssung|willkommen.*stadtwerke|begrüßung.*energieversorger/] },
    { key: 'stadtwerke_sperrung', w: 44, p: [/sperrung.*(strom|gas|stadtwerke)|stadtwerke.*sperrung|energie.*sperrung|strom.*gesperrt/] },
    { key: 'anschlussverguetung', w: 40, p: [/anschlussverg[üu]tung|verg[üu]tung.*einspeisung|einspeiseverg[üu]tung.*stadtwerke|verguetung.*anschluss/] },
    { key: 'vertragsverlaengerung', w: 42, p: [/vertragsverl[aä]ngerung|vertrag.*verl[aä]ngert|verl[aä]ngerung.*vertrag|vertragsverlaengerung/] },
    { key: 'stadtwerke_jahresabschluss', w: 44, p: [/jahresabschluss.*(strom|gas|wasser)|stadtwerke.*jahresabschluss|jahresabschluss.*stadtwerke|jahresabrechnung.*stadtwerke/] },
    { key: 'verbraucherzentrale', w: 36, p: [/verbraucherzentrale|verbraucher.*zentrale|verbraucherberatung.*stadtwerke|energieberatung.*verbraucher/] },
    { key: 'stadtwerke_verbrauchsmeldung', w: 38, p: [/verbrauchsmeldung|z[äa]hlerstand.*melden|z[äa]hlerstand.*mitteilen|verbrauchsabfrage/] },
    { key: 'verbrauchsnachweis', w: 46, p: [/verbrauchsnachweis|nachweis.*verbrauch.*(strom|gas)|verbrauchsbeleg.*abrechnung|z[äa]hlerstandsnachweis/] },
    { key: 'verbrauchseinbruch', w: 40, p: [/verbrauchseinbruch|verbrauch.*[üu]berpr[üu]fen|ungew[öo]hnlich.*verbrauch|verbrauch.*r[üu]ckg[äa]ngig/] },
    { key: 'verbrauchsaenderung', w: 40, p: [/verbrauchs[aä]nderung|verbrauch.*ge[äa]ndert|verbrauch.*[äa]ndert|verbrauchs[aä]nderung.*abrechnung/] },
    { key: 'stadtwerke_widerruf', w: 44, p: [/stadtwerke.*widerruf|widerruf.*stadtwerke|widerruf.*stromvertrag|widerruf.*energievertrag/] },
    { key: 'abschlagsaenderung', w: 36, p: [/abschlags[äa]nderung|abschlag.*[aä]ndern|monatlicher abschlag.*(neu|erh[oö]ht)|abschlag.*neu berechnet/] },
    { key: 'tarifwechsel', w: 38, p: [/tarifwechsel|tarif.*wechsel|tarif.*gewechselt|neuer tarif.*strom|tarif.*umstellen/] },
    { key: 'tarifinfo', w: 48, p: [/tarifinfo|tarif.*information|tarif.*[üu]bersicht|information.*tarif|tarif.*kunden/] },
    { key: 'verbrauchsabschlag', w: 48, p: [/\bverbrauchsabschlag\b/] },
    { key: 'abschlagszahlung', w: 46, p: [/abschlagszahlung|abschlag.*zahlen|zahlung.*abschlag|abschlag.*f[aä]llig/] },
    { key: 'stadtwerke_verbrauchssteuer', w: 48, p: [/verbrauchssteuer.*(strom|gas|wasser)|stromsteuer|energ[iäa]esteuer|kohle[steuer]/] },
    { key: 'stadtwerke_nachzahlung', w: 46, p: [/nachzahlung.*(strom|gas|wasser|energie)|stadtwerke.*nachzahlung|verbrauchsabrechnung.*nachzahlung/] },
    { key: 'mietvertrag', w: 34, p: [/mietvertrag|kaltmiete|warmmiete|mietvertrag.*wohnung/] },
    { key: 'raeumungsvergleich', w: 42, p: [/r[aä]umungsvergleich|r[aä]umung.*vergleich|miet.*r[aä]umung.*vergleich|r[aä]umungs.*auflösung/] },
    { key: 'sicherheitsabschlag', w: 42, p: [/sicherheitsabschlag|sicherheit.*abschlag.*miete|miet.*sicherheitsabschlag|abschlag.*sicherheit.*miete/] },
    { key: 'untervermietung', w: 42, p: [/untervermietung|untermiete|untermieter|vermietung.*unter/] },
    { key: 'mitbewohner', w: 42, p: [/mitbewohner|mitbewohner.*miet|miet.*mitbewohner|einzug.*mitbewohner/] },
    { key: 'mietpreisbremse', w: 42, p: [/mietpreisbremse|preisbremse.*miete|mietpreisbremse.*wohnung|mietpreis.*bremse/] },
    { key: 'indexierung', w: 42, p: [/indexierung.*miete|miete.*indexierung|mietindex.*miete|indexmietvertrag/] },
    { key: 'wohnungsabgabe', w: 42, p: [/wohnungsabgabe|abgabe.*wohnung|wohnung.*abgeben|schl[üu]ssel.*abgeben/] },
    { key: 'nachmieter', w: 42, p: [/nachmieter|nachmieter.*miet|miet.*nachmieter|nachmiet.*[üu]bernehmen/] },
    { key: 'hausordnung', w: 42, p: [/hausordnung|hausordnung.*miet|miet.*hausordnung|hausordnung.*wohnung/] },
    { key: 'mietvertrag_aenderung', w: 42, p: [/mietvertrag.*([äa]nderung|nachtr[aä]glich)|[äa]nderung.*mietvertrag|nachtr[äa]glicher.*(mietvertrag|mieterh[oö]hung)|mietvertrag.*ge[äa]ndert/] },
    { key: 'mietminderung', w: 44, p: [/mietminderung|miete.*gemindert|minderung.*miete|mietmangel.*minderung/] },
    { key: 'mietkaution', w: 42, p: [/mietkaution|kaution.*wohnung|mietkautionsabrechnung|kaution.*[üu]berweisen|kautionsr[üu]ckzahlung/] },
    { key: 'mietvertragsguthaben', w: 42, p: [/guthaben.*miet|miet.*guthaben|mietvertragsguthaben|guthaben.*vermieter/] },
    { key: 'mieterhoehung', w: 44, p: [/mieterh[oö]hung|miete.*erh[oö]ht|erh[oö]hung.*miete|mietanpassung/] },
    { key: 'vermieterwechsel', w: 42, p: [/vermieterwechsel|eigent[üu]merwechsel.*wohnung|neuer vermieter|mieter.*neuer eigent[üu]mer|hausverwaltung.*gewechselt/] },

    // Education & certificates
    { key: 'schulzeugnis', w: 34, p: [/jahreszeugnis|berufsschule|berufskolleg|abschlusszeugnis|abiturzeugnis|schulzeugnis|gesellenbrief|facharbeiterbrief|mittlere reife|hauptschulabschluss|schuljahr/] },
    { key: 'hochschulzeugnis', w: 34, p: [/bachelor of|master of|transcript of records|diplomzeugnis|hochschulabschluss|universit[äa]t.*urkunde|zeugnis.*universit[äa]t/] },
    { key: 'arbeitszeugnis', w: 34, p: [/arbeitszeugnis|zwischenzeugnis|qualifiziertes zeugnis|leistung.*verhalten.*beurteilung|vollsten zufriedenheit/] },
    { key: 'sprachzertifikat', w: 34, p: [/zertifikat deutsch|telc|goethe[- ]?zertifikat|testdaf|deutsch[- ]?test f[iu]r zuwanderer|\bdtz\b|sprachzertifikat/] },
  ];

  // --- Fact extraction -------------------------------------------------------
  const invalidRefs = new Set(['sie','ihr','wir','die','der','das','und','in','an','von','den','dem','mit','fuer','für','herr','frau','noten','beruf','ione','previdenza','sociale','carte','security','card']);

  // Produce normalized variants robust to common OCR digit/letter confusion
  // (1 could be 'i' or 'l'; 0 could be 'o'; 'rn' could be 'm').
  function healOcrSpaces(text) {
    if (!text) return '';
    return text
      // Fix broken German compound words with accidental spaces in OCR
      .replace(/\b(arbeit|ausbildung|vertrag|bescheid|mit|nach|versicher|abrechnung|zeugnis|lohn|steuer|melde|kranken|renten|pflege|dienst|unterlagen|akten)\s+(szeit|svertrag|sdauer|sbescheid|sbeschluss|snummer|sbeleg|skosten|snachweis|sordnung|sort|sende|spflicht|str[äa]ger|szahlung|sstelle|sfall)\b/gi, '$1$2')
      .replace(/\b(arbeit)\s+szeit\b/gi, 'Arbeitszeit')
      .replace(/\b(ausbildung)\s+svertrag\b/gi, 'Ausbildungsvertrag')
      .replace(/\b(ausbildung)\s+sdauer\b/gi, 'Ausbildungsdauer')
      .replace(/\b(ausbildung)\s+sverg[üu]tung\b/gi, 'Ausbildungsvergütung')
      .replace(/\b(jahres)\s+zeugnis\b/gi, 'Jahreszeugnis')
      .replace(/\b(mit)\s+teilung\b/gi, 'Mitteilung')
      .replace(/\b(ab)\s+schluss\b/gi, 'Abschluss')
      .replace(/\b(be)\s+scheid\b/gi, 'Bescheid')
      .replace(/\b(auf)\s+forderung\b/gi, 'Aufforderung')
      .replace(/\b(ein)\s+spruch\b/gi, 'Einspruch')
      .replace(/\b(wider)\_?s?spruch\b/gi, 'Widerspruch')
      .replace(/\b(fest)\s+setzung\b/gi, 'Festsetzung')
      .replace(/\b(sperr)\s+zeit\b/gi, 'Sperrzeit');
  }

  function normalizeVariants(text) {
    const base = healOcrSpaces(text || '')
      .replace(/[\]\[\)\(\|/\\,\.\-\*#_]+/g, ' ')
      .replace(/\b[sS]zeugnis\s+der\s+berufsschule\b/gi, 'Jahreszeugnis der Berufsschule')
      .replace(/\b1ahreszeugnis\b/gi, 'Jahreszeugnis')
      .replace(/\bseverin[cg]\b/gi, 'Carl-Severing-Berufskolleg')
      .toLowerCase();
    const asL = base.replace(/0(?=[a-z])/g, 'o').replace(/(?<=[a-z])0/g, 'o').replace(/1(?=[a-z])/g, 'l').replace(/(?<=[a-z])1/g, 'l').replace(/[\r\n\t]+/g, ' ');
    const asI = base.replace(/0(?=[a-z])/g, 'o').replace(/(?<=[a-z])0/g, 'o').replace(/1(?=[a-z])/g, 'i').replace(/(?<=[a-z])1/g, 'i').replace(/[\r\n\t]+/g, ' ');
    const asRnM = base.replace(/rn/g, 'm').replace(/[\r\n\t]+/g, ' ');
    return [asL, asI, asRnM];
  }

  function normalize(text) {
    return normalizeVariants(text)[0];
  }

  function splitLines(text) {
    return (text || '').split(/[\r\n]+/)
      .map(l => l.replace(/^[,.\-*#\s\d]+/, '').trim())
      .filter(l => l.length > 3 && !/^[,.\-*#\s\d]+$/.test(l));
  }

  function getSubject(lines, norm) {
    let subject = '';
    const titleRe = /\b(berufsausbildungsvertrag|ausbildungsvertrag|lehrvertrag|leihvertrag|sachleihvertrag|[üu]berlassungsvertrag|arbeitsvertrag|anstellungsvertrag|praktikumsvertrag|jahreszeugnis(\s+der\s+berufsschule)?|abschlusszeugnis|halbjahreszeugnis|abiturzeugnis|pr[üu]fungszeugnis|gesellenbrief|facharbeiterbrief|arbeitszeugnis|zwischenzeugnis|f[üu]hrungszeugnis|versicherungsnummernachweis|einkommensteuerbescheid|bu[ss]geldbescheid|mahnbescheid|vollstreckungsbescheid|betriebskostenabrechnung|nebenkostenabrechnung|jahresabrechnung|bewilligungsbescheid|meldebest[äa]tigung|aufenthaltstitel|mitgliedsbescheinigung|arbeitsunf[äa]higkeit)\b/i;
    for (let i = 0; i < Math.min(lines.length, 10); i++) {
      const l = lines[i];
      if (titleRe.test(l)) {
        subject = /jahreszeugnis|berufsschule/i.test(l) ? 'Jahreszeugnis der Berufsschule' : l.trim();
        break;
      }
    }
    if (!subject && /zeugnis|berufsschule|berufskolleg|schuljahr|severing/i.test(norm)) subject = 'Jahreszeugnis der Berufsschule';

    // Explicit Betreff: / subject prefix
    if (!subject) {
      for (let i = 0; i < lines.length; i++) {
        const l = lines[i];
        const m = l.match(/^(?:betreff|betr\.?|angelegenheit|thema|sache)[\s:]*([A-Za-z0-9][\sA-Za-z0-9\-\./äöüÄÖÜß]{3,})/i);
        if (m && m[1].trim().length > 3) { subject = m[1].trim(); break; }
        else if (/^(?:bescheid|mahnung|rechnung|anh[öo]rung|k[üu]ndigung|best[äa]tigung|information|abrechnung|mitteilung|aufforderung|nachweis|anmeldung|strasse|straße)\b/i.test(l)) { subject = l; break; }
      }
    }

    // Salient standalone heading near the top (a short line that reads like a title)
    if (!subject) {
      const docNoun = /bescheid|antrag|anzeige|urkunde|niederschrift|beschluss|genehmigung|anmeldung|abmeldung|auskunft|nachweis|best[äa]tigung|mitteilung|aufforderung|abrechnung|mahnung|verzeichnis|erkl[äa]rung|eingang|erstattung|wiedergabe|sperre|kündigung|kuendigung|steuer|beitrag/i;
      const skipRe = /(^|\s)(an die|hier|amt|stadt|gemeinde|kreis|land|bundes|ministerium|beh[öo]rde|abteilung|referat|dezernat|hausverwaltung|anwalts|kanzlei|bank|kasse|versicher|firma|gmbh|ag\b|muster)(\s|$)/i;
      let titleCaseCandidate = '';
      for (let i = 0; i < Math.min(lines.length, 12); i++) {
        const l = lines[i];
        if (l.length < 3 || l.length > 45) continue;
        if (/\d{2}\.\d{2}\.\d{2,4}/.test(l)) continue;
        if (/^(sehr geehrte|hallo|liebe|die|der|das|wir|sie|ihre|ihr|mit freundlichen|per se|streng)/i.test(l)) continue;
        if (skipRe.test(l)) continue;
        if (docNoun.test(l)) { subject = l.trim(); return subject; }   // strong subject signal
        if (!titleCaseCandidate && /^[A-ZÄÖÜ]/.test(l) && /[a-zäöüß]/.test(l)) titleCaseCandidate = l.trim();
      }
      if (titleCaseCandidate) subject = titleCaseCandidate;
    }

    return subject || 'Amtliches Dokument';
  }

  function getSender(lines, subject) {
    let sender = 'Behörde / Institution';
    if (/severing|carl[- ]?severing/i.test(subject)) return 'Carl-Severing-Berufskolleg';
    for (let i = 0; i < Math.min(lines.length, 8); i++) {
      const l = lines[i];
      if (l === subject) continue;
      if (/schule|berufsschule|kolleg|gymnasium|universit|akademie|ihk|hwk|freistaat|land|amt|ministerium|bund|kasse|service|gericht|polizei|verwaltung|versicher|werke|gmbh|ag\b|kanzlei|klinik|gesellschaft|bank|sparkasse/i.test(l)) { sender = l; break; }
    }
    return sender;
  }

  function getReference(text) {
    const refMatch = text.match(/\b(?:aktenzeichen|az|gesch[äa]ftszeichen|kundennummer|kunden-nr|rechnungsnummer|rechnungs-nr|beleg-nr|vertragsnummer|vertrags-nr|matrikelnummer|sch[üu]lernummer|versichertennummer|mitgliedsnummer|vorgangsnummer)\b[\s:]*([A-Za-z0-9\/\-_. ]{3,30})/i);
    if (refMatch) {
      const cand = refMatch[1].trim();
      if (!invalidRefs.has(cand.toLowerCase()) && cand.length > 2 && !/previdenza|sociale/i.test(cand)) return cand;
    }
    return '—';
  }

  function getAmount(text) {
    const m = text.match(/([0-9]+[.,][0-9]{2})\s*(?:€|EUR|Euro)/i);
    return m ? m[1].replace(',', '.') + ' €' : null;
  }

  function getDeadline(text) {
    const m = text.match(/(?:frist bis|bis sp[äa]testens zum|sp[äa]testens zum|sp[äa]testens am|sp[äa]testens bis|f[äa]llig am|zahlbar bis|bis zum|zum)\s*([0-3]?[0-9]\.[0-1]?[0-9]\.20[2-3][0-9])/i);
    if (!m) return { deadline: null, daysRemaining: null };
    const deadline = m[1].trim();
    const parts = deadline.split('.');
    const target = new Date(+parts[2], +parts[1] - 1, +parts[0]);
    const diff = Math.ceil((target - Date.now()) / 86400000);
    return { deadline, daysRemaining: diff >= 0 ? diff : 0 };
  }

  // --- Public API ------------------------------------------------------------
  function classify(text) {
    const clean = (text || '').trim();
    const norms = normalizeVariants(clean);
    const norm = norms[0];
    const lines = splitLines(clean);
    const subject = getSubject(lines, norm);
    const sender = getSender(lines, subject);
    const reference = getReference(clean);
    const amount = getAmount(clean);
    const { deadline, daysRemaining } = getDeadline(clean);

    const scores = {};
    for (const r of RULES) {
      if (!r.p.length) continue;
      let s = 0;
      for (const pattern of r.p) {
        const matched = typeof pattern === 'string'
          ? norms.some(n => n.includes(pattern))
          : norms.some(n => pattern.test(n));
        if (matched) s += r.w;
      }
      if (s > 0) scores[r.key] = (scores[r.key] || 0) + s;
    }

    let bestKey = 'general';
    let maxScore = 0;
    for (const [k, sc] of Object.entries(scores)) {
      if (sc > maxScore) { maxScore = sc; bestKey = k; }
    }
    const docKey = maxScore >= 5 ? bestKey : 'general';

    return {
      docKey,
      subject,
      sender,
      reference,
      amount,
      deadline,
      daysRemaining,
      scores,
      isInformational: deadline === null && amount === null,
    };
  }

  return { classify, RULES };
});