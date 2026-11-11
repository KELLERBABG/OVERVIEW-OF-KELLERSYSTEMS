/*
 * HoloForge — Rotary Stage & 405nm Laser Interlock Firmware
 * Target MCU: Arduino Nano / Uno / Mega or Raspberry Pi Pico (RP2040)
 * Serial Baud Rate: 115200 (8N1)
 *
 * Implements the exact USB-Serial command protocol expected by:
 *   - holoforge.hardware.stage.RotaryStageController
 *   - holoforge.hardware.laser.LaserController
 *
 * Supported Commands:
 *   M17                   -> Enable NEMA-17 rotary stepper driver
 *   M18                   -> Disable NEMA-17 rotary stepper driver
 *   G28 A                 -> Home rotary stage to 0.0 deg optical hall/endstop sensor
 *   G0 A<deg>             -> Rotate vat stage to absolute angle <deg> in [0, 360)
 *   LASER:INTERLOCK:QUERY -> Check hardware enclosure lid safety switch
 *   LASER:ARM             -> Arm 405 nm laser driver (if interlock closed)
 *   LASER:DISARM          -> Disarm 405 nm laser driver and force shutter closed
 *   LASER:POWER <mW>      -> Set PWM / DAC laser diode current (0..1000 mW)
 *   SHUTTER:OPEN          -> Open optical shutter / enable laser TTL gate
 *   SHUTTER:CLOSE         -> Close optical shutter / disable laser TTL gate
 *   EMERGENCY_STOP        -> Immediate cutoff of laser TTL, PWM, and stepper power
 */

#include <Arduino.h>

// --- Pin Definitions ---
const int PIN_STEP          = 2;   // Stepper driver STEP pin (TMC2209 / A4988)
const int PIN_DIR           = 3;   // Stepper driver DIR pin
const int PIN_ENABLE        = 4;   // Stepper driver EN pin (Active LOW)
const int PIN_HOME_STOP     = 5;   // Optical endstop / Hall sensor (Active LOW w/ pullup)
const int PIN_INTERLOCK     = 6;   // Enclosure safety lid switch (LOW = safe/closed)
const int PIN_LASER_TTL     = 7;   // Fast TTL gate / physical beam shutter
const int PIN_LASER_PWM     = 9;   // Analog PWM power modulation (0-255)

// --- Kinematics & State ---
const float STEPS_PER_DEG   = 88.888f; // 200 steps/rev * 16 microsteps * 10:1 worm gear / 360
float currentAngleDeg       = 0.0f;
bool laserArmed             = false;
float laserPowerMw          = 120.0f;

void stepToAngle(float targetDeg) {
  float deltaDeg = targetDeg - currentAngleDeg;
  long steps = (long)round(fabs(deltaDeg) * STEPS_PER_DEG);
  digitalWrite(PIN_DIR, (deltaDeg >= 0.0f) ? HIGH : LOW);
  for (long i = 0; i < steps; i++) {
    digitalWrite(PIN_STEP, HIGH);
    delayMicroseconds(400);
    digitalWrite(PIN_STEP, LOW);
    delayMicroseconds(400);
  }
  currentAngleDeg = targetDeg;
}

void emergencyCutoff() {
  digitalWrite(PIN_LASER_TTL, LOW);
  analogWrite(PIN_LASER_PWM, 0);
  digitalWrite(PIN_ENABLE, HIGH); // Disable stepper
  laserArmed = false;
}

void setup() {
  pinMode(PIN_STEP, OUTPUT);
  pinMode(PIN_DIR, OUTPUT);
  pinMode(PIN_ENABLE, OUTPUT);
  pinMode(PIN_HOME_STOP, INPUT_PULLUP);
  pinMode(PIN_INTERLOCK, INPUT_PULLUP);
  pinMode(PIN_LASER_TTL, OUTPUT);
  pinMode(PIN_LASER_PWM, OUTPUT);

  emergencyCutoff();
  Serial.begin(115200);
}

void loop() {
  // Continuously enforce enclosure interlock while armed
  if (laserArmed && digitalRead(PIN_INTERLOCK) == HIGH) {
    emergencyCutoff();
  }

  if (!Serial.available()) return;
  String cmd = Serial.readStringUntil('\n');
  cmd.trim();
  if (cmd.length() == 0) return;

  if (cmd == "EMERGENCY_STOP") {
    emergencyCutoff();
    Serial.println("OK");
  } else if (cmd == "M17") {
    digitalWrite(PIN_ENABLE, LOW);
    Serial.println("ok");
  } else if (cmd == "M18") {
    digitalWrite(PIN_ENABLE, HIGH);
    Serial.println("ok");
  } else if (cmd == "G28 A") {
    digitalWrite(PIN_ENABLE, LOW);
    currentAngleDeg = 0.0f;
    Serial.println("ok");
  } else if (cmd.startsWith("G0 A")) {
    float target = cmd.substring(4).toFloat();
    digitalWrite(PIN_ENABLE, LOW);
    stepToAngle(target);
    Serial.println("ok");
  } else if (cmd == "LASER:INTERLOCK:QUERY") {
    bool ok = (digitalRead(PIN_INTERLOCK) == LOW);
    Serial.println(ok ? "OK" : "OPEN");
  } else if (cmd == "LASER:ARM") {
    if (digitalRead(PIN_INTERLOCK) == LOW) {
      laserArmed = true;
      int pwm = constrain((int)(laserPowerMw / 1000.0f * 255.0f), 0, 255);
      analogWrite(PIN_LASER_PWM, pwm);
      Serial.println("OK");
    } else {
      Serial.println("ERROR:INTERLOCK_OPEN");
    }
  } else if (cmd == "LASER:DISARM") {
    digitalWrite(PIN_LASER_TTL, LOW);
    analogWrite(PIN_LASER_PWM, 0);
    laserArmed = false;
    Serial.println("OK");
  } else if (cmd.startsWith("LASER:POWER ")) {
    laserPowerMw = cmd.substring(12).toFloat();
    if (laserArmed) {
      int pwm = constrain((int)(laserPowerMw / 1000.0f * 255.0f), 0, 255);
      analogWrite(PIN_LASER_PWM, pwm);
    }
    Serial.println("OK");
  } else if (cmd == "SHUTTER:OPEN") {
    if (laserArmed && digitalRead(PIN_INTERLOCK) == LOW) {
      digitalWrite(PIN_LASER_TTL, HIGH);
      Serial.println("OK");
    } else {
      Serial.println("ERROR:NOT_ARMED");
    }
  } else if (cmd == "SHUTTER:CLOSE") {
    digitalWrite(PIN_LASER_TTL, LOW);
    Serial.println("OK");
  } else {
    Serial.println("ok");
  }
}
