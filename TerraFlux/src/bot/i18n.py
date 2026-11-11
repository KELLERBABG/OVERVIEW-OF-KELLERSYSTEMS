"""Grower facing message catalogue (English / German).

The catalogue holds the *complete* message templates per language. The WhatsApp
formatter renders them as markdown, the Telegram formatter converts the markdown to
HTML (bold plus HTML escaping), so both channels always show the same wording.

Unknown keys fall back to English, and a missing language falls back to English as
well, so a partially translated catalogue can never break a message.
"""
import html
import re
from typing import Any, Dict

LANGUAGES = ('en', 'de')

CATALOGUE: Dict[str, Dict[str, str]] = {
    'en': {
        'advisory.irrigate': (
            "💧 *TerraFlux Daily Advisory (FieldSense Engine)*\n"
            "📍 *{field}*{scope} ({crop} • {area}ha)\n\n"
            "🌱 *Soil Moisture:* {moisture}% (Threshold: {threshold}%)\n"
            "☀️ *Tomorrow's ET₀:* {et0}mm | *Rain:* {rain}mm\n\n"
            "👉 *ACTION:* Run the drip for *{gross}mm* tomorrow at *{time}*\n"
            "💧 *Root zone refill:* {net}mm net\n"
            "⏱️ *Drip Run Time:* ~{duration}\n\n"
            "_Reply *DONE* when finished, or *DELAY 1D* to postpone._"
        ),
        'advisory.no_water': (
            "✅ *TerraFlux Daily Advisory (FieldSense Engine)*\n"
            "📍 *{field}*{scope} ({crop} • {area}ha)\n\n"
            "🌱 *Soil Moisture:* {moisture}% (Healthy reserve)\n"
            "☀️ *Tomorrow's ET₀:* {et0}mm | *Rain Forecast:* {rain}mm\n\n"
            "👉 *ACTION:* *NO WATERING NEEDED*\n"
            "💡 _{reason}_\n\n"
            "_Reply *STATUS* anytime for live telemetry._"
        ),
        'status': (
            "📊 *TerraFlux | FieldSense Live Status: {field}*\n"
            "🌱 *Current Moisture:* {moisture}% VWC\n"
            "🌡️ *Soil Temp:* {temp}°C\n"
            "🔋 *Probe Battery:* {battery}V (Healthy)\n"
            "📡 *Last Uplink:* {uplink} UTC\n\n"
            "📌 *Status:* {state}\n\n"
            "Commands: *DONE* | *WATER 12mm* | *DELAY 1D* | *HELP*"
        ),
        'confirm': (
            "✅ *Logged {depth}mm Irrigation for {field}*\n"
            "⏱️ Duration: ~{minutes} minutes recorded.\n"
            "🌱 Soil water reservoir model updated. Good job conserving water!"
        ),
        'delay': (
            "⏱️ *Acknowledged.* Irrigation alert for *{field}* postponed for {days} day(s). "
            "Will resume checking on {date}."
        ),
        'help': (
            "🌾 *TerraFlux (FieldSense Engine) Commands:*\n"
            "• *DONE* / *OK* — Log scheduled watering completed\n"
            "• *STATUS* — Get live soil moisture & battery health\n"
            "• *WATER <mm>* — Record custom irrigation (e.g. `WATER 15mm`)\n"
            "• *DELAY 1D* — Postpone irrigation reminder by 24 hours\n"
            "• *PAUSE 3D* — Silence alerts for 3 days (e.g. spraying/harvest)"
        ),
        'digest': (
            "📊 *TerraFlux Weekly Summary — {field}*\n"
            "🌱 Soil moisture now: {moisture}% (threshold {threshold}%)\n"
            "💧 Water applied: {applied}mm net in {irrigations} irrigation(s)\n"
            "☀️ Crop demand (ET₀) over the week: {et0}mm\n"
            "🌧️ Rain over the week: {rain}mm\n"
            "💬 Reply *STATUS* for live telemetry, *HELP* for all commands."
        ),
        'escalation': (
            "🚨 *TerraFlux escalation — {field}*\n"
            "The recommendation of {age} h ago is still unconfirmed: {reason}\n"
            "🌱 Soil moisture: {moisture}% (threshold {threshold}%)\n"
            "👉 *ACTION:* {action}\n"
            "Please check the block or reply *DONE* once the irrigation ran."
        ),
        'state.optimal': 'Soil moisture optimal (above {threshold}% threshold)',
        'state.approaching': 'Approaching the RAW threshold ({threshold}%)',
        'state.stressed': 'Below the RAW threshold ({threshold}%) - irrigation required',
        'state.no_data': 'No probe data available'
    },
    'de': {
        'advisory.irrigate': (
            "💧 *TerraFlux Tagesempfehlung (FieldSense Engine)*\n"
            "📍 *{field}*{scope} ({crop} • {area}ha)\n\n"
            "🌱 *Bodenfeuchte:* {moisture}% (Grenzwert: {threshold}%)\n"
            "☀️ *ET₀ morgen:* {et0}mm | *Regen:* {rain}mm\n\n"
            "👉 *AKTION:* Tropfschlauch für *{gross}mm* morgen um *{time}* laufen lassen\n"
            "💧 *Auffüllung der Wurzelzone:* {net}mm netto\n"
            "⏱️ *Laufzeit:* ~{duration}\n\n"
            "_Antworte mit *DONE*, wenn fertig, oder *DELAY 1D* zum Verschieben._"
        ),
        'advisory.no_water': (
            "✅ *TerraFlux Tagesempfehlung (FieldSense Engine)*\n"
            "📍 *{field}*{scope} ({crop} • {area}ha)\n\n"
            "🌱 *Bodenfeuchte:* {moisture}% (gesundes Niveau)\n"
            "☀️ *ET₀ morgen:* {et0}mm | *Regenprognose:* {rain}mm\n\n"
            "👉 *AKTION:* *KEIN BESSERN NÖTIG*\n"
            "💡 _{reason}_\n\n"
            "_Antworte jederzeit mit *STATUS* für aktuelle Messwerte._"
        ),
        'status': (
            "📊 *TerraFlux | FieldSense Live-Status: {field}*\n"
            "🌱 *Aktuelle Feuchte:* {moisture}% VWC\n"
            "🌡️ *Bodentemperatur:* {temp}°C\n"
            "🔋 *Batterie der Sonde:* {battery}V (gut)\n"
            "📡 *Letztes Uplink:* {uplink} UTC\n\n"
            "📌 *Status:* {state}\n\n"
            "Befehle: *DONE* | *WATER 12mm* | *DELAY 1D* | *HELP*"
        ),
        'confirm': (
            "✅ *{depth}mm Bewässerung für {field} erfasst*\n"
            "⏱️ Dauer: ~{minutes} Minuten gebucht.\n"
            "🌱 Bodenwassermodell aktualisiert. Gut, Wasser zu sparen!"
        ),
        'delay': (
            "⏱️ *Bestätigt.* Bewässerungsempfehlung für *{field}* um {days} Tag(e) "
            "verschoben. Wir prüfen ab {date} wieder."
        ),
        'help': (
            "🌾 *TerraFlux (FieldSense Engine) Befehle:*\n"
            "• *DONE* / *OK* — Bewässerung als erledigt melden\n"
            "• *STATUS* — Aktuelle Bodenfeuchte und Batteriestatus\n"
            "• *WATER <mm>* — Manuelle Bewässerung erfassen (z. B. `WATER 15mm`)\n"
            "• *DELAY 1D* — Erinnerung um 24 Stunden verschieben\n"
            "• *PAUSE 3D* — Benachrichtigungen 3 Tage aussetzen (Spritzen/Ernte)"
        ),
        'digest': (
            "📊 *TerraFlux Wochenübersicht — {field}*\n"
            "🌱 Bodenfeuchte jetzt: {moisture}% (Grenzwert {threshold}%)\n"
            "💧 Ausgebrachtes Wasser: {applied}mm netto in {irrigations} Bewässerung(en)\n"
            "☀️ Kulturbedarf (ET₀) der Woche: {et0}mm\n"
            "🌧️ Regen der Woche: {rain}mm\n"
            "💬 Antworte mit *STATUS* für aktuelle Messwerte, *HELP* für alle Befehle."
        ),
        'escalation': (
            "🚨 *TerraFlux Eskalation — {field}*\n"
            "Die Empfehlung von vor {age} h ist noch unbestätigt: {reason}\n"
            "🌱 Bodenfeuchte: {moisture}% (Grenzwert {threshold}%)\n"
            "👉 *AKTION:* {action}\n"
            "Bitte die Parzelle prüfen oder mit *DONE* bestätigen, dass die Bewässerung lief."
        ),
        'state.optimal': 'Bodenfeuchte optimal (über {threshold}% Grenzwert)',
        'state.approaching': 'Nähert sich dem RAW-Grenzwert ({threshold}%)',
        'state.stressed': 'Unter dem RAW-Grenzwert ({threshold}%) - Bewässerung nötig',
        'state.no_data': 'Keine Sensordaten vorhanden'
    }
}


def normalize_language(language: str) -> str:
    """Map a language hint onto a supported catalogue ('de-DE' -> 'de')."""
    if not language:
        return 'en'
    code = str(language).strip().lower().replace('_', '-').split('-')[0]
    return code if code in LANGUAGES else 'en'


def translate(key: str, language: str = 'en', **kwargs: Any) -> str:
    """Render a catalogue entry. Falls back to English and finally to the key."""
    lang = normalize_language(language)
    template = CATALOGUE.get(lang, {}).get(key) or CATALOGUE['en'].get(key)
    if template is None:
        return key
    try:
        return template.format(**kwargs)
    except (KeyError, IndexError):
        return template


def to_telegram_html(markdown_text: str) -> str:
    """Convert a rendered markdown message into Telegram HTML.

    The catalogue keeps the wording channel independent: markdown for WhatsApp, and
    this converter produces the HTML that the Telegram Bot API expects.
    """
    escaped = html.escape(markdown_text, quote=False)
    # *bold* -> <b>bold</b> and _italic_ -> <i>italic</i>
    escaped = re.sub(r'\*([^*\n]+)\*', r'<b>\1</b>', escaped)
    escaped = re.sub(r'(?<!\w)_([^_\n]+)_(?!\w)', r'<i>\1</i>', escaped)
    # The inline code markers are not supported by Telegram HTML - drop them.
    return escaped.replace('`', '')
