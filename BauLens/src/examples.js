export const GOOD_EXAMPLE = {
  projekt: "Musterweg 12, 80331 München",
  grundstueck: {
    flaeche_m2: 800,
    grenzabstaende: { norden: 5.0, osten: 4.0, sueden: 4.0, westen: 4.0 },
  },
  gebaeude: {
    hoehe_m: 7.0,
    geschosse_ueber_grund: 2,
    geschosse_unter_ground: 1,
    wohnungen_anzahl: 2,
    nutzflaeche_m2: 210,
    grz_ist: 0.35,
    gfz_ist: 0.70,
    stellplaetze_ist: 2,
    fahrradstellplaetze_ist: 4,
    aufenthaltsraeume: [
      { name: "Wohnen/Essen EG", flaeche_m2: 32, fensterflaeche_m2: 4.6, raumhoehe_m: 2.60, etage: 0 },
      { name: "Schlafen OG", flaeche_m2: 18, fensterflaeche_m2: 2.5, raumhoehe_m: 2.55, etage: 1 },
      { name: "Kind OG", flaeche_m2: 14, fensterflaeche_m2: 1.8, raumhoehe_m: 2.50, etage: 1 },
    ],
    zweiter_rettungsweg_vorhanden: true,
    treppenraum_vorhanden: true,
  },
  einstellungen: { grz_limit: 0.4, gfz_limit: 1.2 },
};

export const BAD_EXAMPLE = {
  projekt: "Am Hang 3, 70173 Stuttgart",
  grundstueck: {
    flaeche_m2: 600,
    grenzabstaende: { norden: 3.0, sueden: 2.5 },
  },
  gebaeude: {
    hoehe_m: 15.0,
    geschosse_ueber_grund: 4,
    geschosse_unter_ground: 0,
    wohnungen_anzahl: 8,
    grz_ist: 0.52,
    gfz_ist: 1.80,
    stellplaetze_ist: 2,
    aufenthaltsraeume: [
      { name: "Wohnraum EG", flaeche_m2: 40, fensterflaeche_m2: 3.0, raumhoehe_m: 2.10, etage: 0 },
      { name: "Schlafzimmer OG", flaeche_m2: 20, fensterflaeche_m2: 2.0, raumhoehe_m: 2.30, etage: 1 },
    ],
    zweiter_rettungsweg_vorhanden: false,
    treppenraum_vorhanden: false,
  },
  einstellungen: { grz_limit: 0.4, gfz_limit: 1.2 },
};
