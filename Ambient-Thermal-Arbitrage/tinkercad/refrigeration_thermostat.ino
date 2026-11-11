/* =====================================================================
   Ambient Thermal Arbitrage (ATA) - Tinkercad refrigeration thermostat
   ---------------------------------------------------------------------
   Prototype of the ATA control engine (project: Ambient Thermal
   Arbitrage / thermal-arbitrage-mvp) running on an Arduino Uno R3 in
   Tinkercad Circuits.

   The Uno plays BOTH roles:
     (1) the cold-room "plant" - a small RC-style thermal model that
         warms with the load and cools when the compressor runs, and
     (2) the ATA control engine - emergency/fail-safe, ThermalGuard,
         GridBalancing, pre-cool/Arbitrage, normal thermostat, all gated
         by the anti-short-cycle governor.

   Reference (real system):
     thermal-arbitrage-mvp/src/control/mod.rs   (decide / governor_lock)
     thermal-arbitrage-mvp/config.toml          (parameters below)
   ===================================================================== */

#include <LiquidCrystal.h>
#include <stdlib.h>
#include <math.h>

/* ---------- LCD: LiquidCrystal(RS, E, DB4, DB5, DB6, DB7) ---------- */
LiquidCrystal lcd(12, 11, 5, 4, 3, 2);

/* ---------- Pin map ---------- */
const uint8_t PIN_TMP36     = A0; // ambient (outside) temperature
const uint8_t PIN_POT_PRICE = A1; // virtual electricity price 0..120 EUR/MWh
const uint8_t PIN_POT_FREQ  = A2; // grid frequency deviation -0.5..+0.5 Hz
const uint8_t PIN_BTN_DOOR  = 6;  // door-open disturbance (INPUT_PULLUP -> GND)
const uint8_t PIN_PIEZO     = 7;  // alarm buzzer
const uint8_t PIN_LED_COMP  = 8;  // compressor
const uint8_t PIN_LED_FAN   = 9;  // evaporator fan
const uint8_t PIN_LED_ALARM = 10; // emergency alarm

/* ---------- ATA parameters (config.toml) ---------- */
const float T_MIN          = -24.0;
const float T_MAX          = -18.0;
const float T_SETPOINT     = -21.0;
const float T_SETPOINT_MIN = -24.0;
const float T_SETPOINT_MAX = -18.0;

const float MIN_RUNTIME_S        = 900.0;  // anti-short-cycle: min run
const float MIN_OFF_S            = 600.0;  // anti-short-cycle: min off
const float GRID_DEADBAND_HZ     = 0.1;
const float MPC_CHEAP_EUR        = 40.0;
const float THERMAL_INTEGRAL_MAX = 3600.0;

/* ---------- Demo time compression ----------
   1 real second == TIME_SCALE simulated seconds, so 900 s of real
   industrial timing is watchable in ~15 s. ---------------- */
const float TIME_SCALE = 60.0;

/* The emergency "time-to-failure" trip is scaled down with the demo plant:
   the real 900 s assumes the ~17-day cold-store tau, which the demo plant
   cannot reproduce. The trip LOGIC is identical to the real engine. */
const float TTF_CRITICAL_S = 400.0;

/* ---------- Demo plant (scaled stand-ins for r_th/c_th and the
     cooling / heat-leak power of the 1000 m3 cold store) ------- */
const float LEAK_K      = 0.00008; // dC per sim-s per dC of (Tamb - Tin)
const float COOL_K      = 0.0060;  // dC per sim-s of active cooling
const float DOOR_K      = 0.0080;  // extra leak while the door is held open
const float FAN_RUNON_S = 45.0;    // evaporator fan run-on after comp stops

/* ---------- State ---------- */
float tInternal = -21.0;
float tAmbient  = 25.0;
float price     = 50.0;

bool compOn = false, prevCompOn = false;
bool fanOn  = false;
float lastCompOffSim = -1e9;

bool  lockActive = false, lockState = false, lastState = false;
float lockUntil = 0.0;
bool  emergencyLatched = false;
float thermalIntegral = 0.0;
float energyWh = 0.0;

float simTime = 0.0;
unsigned long lastMs = 0, lastLcdMs = 0, lastLogMs = 0;
bool doorOpen = false;

enum Mode { M_IDLE, M_ARBIT, M_GRID, M_MPC, M_GUARD, M_EMERG };
Mode mode = M_IDLE;

/* NOTE: the parameter is `int`, not `Mode`. The Arduino/Tinkercad builder
   auto-generates function prototypes and inserts them ABOVE the sketch, i.e.
   before this `enum` is declared. A signature using the custom `Mode` type
   therefore fails with "'Mode' was not declared in this scope". Plain `int`
   keeps the auto-prototype valid; the enum constants still work as cases. */
const char* modeName(int m) {
  switch (m) {
    case M_ARBIT: return "ARBIT";
    case M_GRID:  return "GRID";
    case M_MPC:   return "MPC";
    case M_GUARD: return "GUARD";
    case M_EMERG: return "EMERG";
    default:      return "IDLE";
  }
}

/* ================= setup ================= */
void setup() {
  pinMode(PIN_BTN_DOOR, INPUT_PULLUP);
  pinMode(PIN_PIEZO, OUTPUT);
  pinMode(PIN_LED_COMP, OUTPUT);
  pinMode(PIN_LED_FAN, OUTPUT);
  pinMode(PIN_LED_ALARM, OUTPUT);
  lcd.begin(16, 2);
  Serial.begin(9600);
  Serial.println(F("ATA refrigeration thermostat - Tinkercad prototype"));
  Serial.println(F("sim_s,Tin,Tamb,price,freq_dev,mode,comp,fan,setpoint,lock_s,kWh,cost_eur"));
  lastMs = millis();
}

/* ================= helpers ================= */
/* Anti-short-cycle governor: a state change starts a min run / min off
   lock; the locked state wins until it expires (mirrors governor_lock). */
bool governorLock(bool desired, float t) {
  if (lockActive) {
    if (t < lockUntil) return lockState;
    lockActive = false;
  }
  if (desired != lastState) {
    lastState = desired;
    lockActive = true;
    lockState = desired;
    lockUntil = t + (desired ? MIN_RUNTIME_S : MIN_OFF_S);
    return lockState;
  }
  return desired;
}

void lcdLine(uint8_t row, const char* s) {
  lcd.setCursor(0, row);
  lcd.print(F("                "));
  lcd.setCursor(0, row);
  lcd.print(s);
}

/* ================= loop ================= */
void loop() {
  unsigned long now = millis();
  float dtReal = (now - lastMs) / 1000.0;
  lastMs = now;
  if (dtReal < 0.0 || dtReal > 0.5) dtReal = 0.1;   // guard against stalls
  float dtSim = dtReal * TIME_SCALE;
  simTime += dtSim;

  /* ---------- inputs ---------- */
  float mv = analogRead(PIN_TMP36) * (5000.0 / 1024.0);
  tAmbient = (mv - 500.0) / 10.0;                  // TMP36: 500 mV @ 0 C, 10 mV/C

  price = analogRead(PIN_POT_PRICE) * (120.0 / 1023.0);
  float dev = (analogRead(PIN_POT_FREQ) / 1023.0 - 0.5) * 1.0;   // -0.5..+0.5 Hz
  doorOpen = (digitalRead(PIN_BTN_DOOR) == LOW);

  /* ---------- plant: advance the cold-room temperature ---------- */
  float leak = LEAK_K * (tAmbient - tInternal) + (doorOpen ? DOOR_K : 0.0);
  float cool = compOn ? COOL_K : 0.0;
  tInternal += (leak - cool) * dtSim;
  tInternal = constrain(tInternal, -40.0, 60.0);

  float setpoint = T_SETPOINT;
  mode = M_IDLE;

  /* ---------- priority 1: emergency (fail-safe cooling) ---------- */
  if (emergencyLatched && tInternal <= T_SETPOINT) emergencyLatched = false;
  float ttf = 1e9;
  if (tInternal >= T_MAX) {
    ttf = 0.0;
  } else if (!compOn) {
    float rate = LEAK_K * (tAmbient - tInternal) + (doorOpen ? DOOR_K : 0.0);
    if (rate > 0.000001) ttf = (T_MAX - tInternal) / rate;
  }
  bool emergency = emergencyLatched || (ttf < TTF_CRITICAL_S) || (tInternal >= T_MAX);

  if (emergency) {
    emergencyLatched = true;
    if (!lastState) {                 // record ON switch so the governor
      lastState = true;               // cannot force OFF mid-emergency
      lockActive = true; lockState = true;
      lockUntil = simTime + MIN_RUNTIME_S;
    }
    compOn = true;
    setpoint = T_SETPOINT_MIN;
    mode = M_EMERG;
  } else {
    /* ---------- raw decision (same priority order as decide()) ---------- */
    bool want = false;

    if (thermalIntegral > THERMAL_INTEGRAL_MAX) {
      want = true; setpoint = T_SETPOINT; mode = M_GUARD;            // ThermalGuard
    } else if (fabs(dev) > GRID_DEADBAND_HZ) {
      bool w = (dev > 0.0);                     // +Hz => absorb (cool) / -Hz => shed
      if (w && tInternal <= T_MIN)        w = false;   // never over-cool
      if (!w && tInternal >= T_MAX - 1.0) w = true;    // never shed near the limit
      want = w;
      setpoint = (dev > 0.0) ? T_SETPOINT_MIN : T_SETPOINT_MAX;
      mode = M_GRID;
    } else if (price < MPC_CHEAP_EUR && tInternal > T_MIN) {
      want = true; setpoint = T_SETPOINT_MIN; mode = M_MPC;          // cheap window pre-cool
    } else if (tInternal > T_MIN && tInternal < T_MAX && price > 80.0) {
      want = false; setpoint = T_SETPOINT_MAX;                       // peak price shed
      mode = M_ARBIT;
    } else {
      want = compOn ? (tInternal > T_SETPOINT - 1.0)     // thermostat + hysteresis
                    : (tInternal >= T_SETPOINT + 1.0);
      mode = M_IDLE;
    }

    /* ---------- anti-short-cycle governor ---------- */
    compOn = governorLock(want, simTime);
    if (lockActive && lockState) setpoint = T_SETPOINT_MIN;   // ThermalGuard clamp
  }

  /* ---------- thermal excursion integral ---------- */
  float excess = tInternal - T_SETPOINT;
  if (excess > 0.0) thermalIntegral += excess * dtSim;
  else              thermalIntegral *= exp(-dtSim / 600.0);
  if (thermalIntegral > THERMAL_INTEGRAL_MAX * 2.0)
    thermalIntegral = THERMAL_INTEGRAL_MAX * 2.0;

  /* ---------- energy + cost ---------- */
  if (compOn) energyWh += (50000.0 * dtSim) / 3600.0;   // 50 kW compressor

  /* ---------- evaporator fan run-on ---------- */
  if (prevCompOn && !compOn) lastCompOffSim = simTime;
  fanOn = compOn || (simTime - lastCompOffSim < FAN_RUNON_S);
  prevCompOn = compOn;

  /* ---------- outputs ---------- */
  digitalWrite(PIN_LED_COMP, compOn ? HIGH : LOW);
  digitalWrite(PIN_LED_FAN,  fanOn  ? HIGH : LOW);
  bool alarm = (mode == M_EMERG);
  digitalWrite(PIN_LED_ALARM, alarm ? HIGH : LOW);
  if (alarm) tone(PIN_PIEZO, 2000); else noTone(PIN_PIEZO);

  /* ---------- LCD (4 Hz) ---------- */
  if (now - lastLcdMs >= 250) {
    lastLcdMs = now;
    char ts[8], ss[8], b0[20], b1[20];
    dtostrf(tInternal, 5, 1, ts);      // "-20.6"
    dtostrf(setpoint, 5, 1, ss);       // "-21.0"
    snprintf(b0, sizeof(b0), "T%s SET%s", ts, ss);            // 15 chars
    int lockRemain = lockActive ? (int)(lockUntil - simTime) : 0;
    snprintf(b1, sizeof(b1), "%5s C:%s L%3d",
             modeName(mode), compOn ? "ON " : "OFF", lockRemain); // 16 chars
    lcdLine(0, b0);
    lcdLine(1, b1);
  }

  /* ---------- serial log (1 Hz) ---------- */
  if (now - lastLogMs >= 1000) {
    lastLogMs = now;
    Serial.print(simTime, 0);             Serial.print(',');
    Serial.print(tInternal, 1);           Serial.print(',');
    Serial.print(tAmbient, 1);            Serial.print(',');
    Serial.print(price, 0);               Serial.print(',');
    Serial.print(dev, 2);                 Serial.print(',');
    Serial.print(modeName(mode));         Serial.print(',');
    Serial.print(compOn ? "ON" : "OFF");  Serial.print(',');
    Serial.print(fanOn ? "ON" : "OFF");   Serial.print(',');
    Serial.print(setpoint, 0);            Serial.print(',');
    Serial.print(lockActive ? (lockUntil - simTime) : 0.0, 0); Serial.print(',');
    Serial.print(energyWh / 1000.0, 3);   Serial.print(',');
    Serial.println((energyWh / 1000.0) * price, 3);
  }

  delay(100);
}
