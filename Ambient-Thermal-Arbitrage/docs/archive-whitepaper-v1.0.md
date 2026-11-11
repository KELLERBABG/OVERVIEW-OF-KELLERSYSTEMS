# Ambient Thermal Arbitrage

> **⚠️ SUPERSEDED (2026-08-11):** This v1.0 document is historical. The current
> technical whitepaper is `../../whitepaper-ATA.md` (v3.0) at the repository
> root. The "Five-Mode Priority Cascade" described here has been superseded by
> the 6-mode (gateway) / 7-mode (fleet) priority cascades.

## Model Predictive Control for Industrial Refrigeration Load Management

### Technical White Paper — Version 1.0

---

## 1. Executive Summary

**Ambient Thermal Arbitrage** is a decision-guidance system for industrial refrigeration and cold storage facilities. It leverages the inherent thermal inertia of refrigerated spaces — the fact that temperature changes slowly thanks to insulation, product mass, and structural thermal capacitance — to shift compressor electricity consumption away from expensive, grid-strained periods and into cheaper, lower-demand windows.

The system continuously ingests real-time electricity market prices, ambient weather conditions, and grid frequency measurements. A cascading five-mode Model Predictive Controller (MPC) evaluates these inputs against the facility's thermal state and physical constraints to determine the optimal compressor operating strategy. The overarching objective is to reduce electricity costs without compromising the stored product's temperature integrity.

This document describes the system's architecture, core algorithms, and operational principles. It is intended for energy managers, facility operators, and technical stakeholders evaluating demand-side flexibility solutions for cold chain infrastructure.

---

## 2. Problem Statement

### 2.1 The Cost of Cold

Industrial refrigeration is one of the largest consumers of electricity in the food and logistics sectors. A typical medium-sized cold storage facility (≈1,000 m³, 200 tonnes of product) may draw 50 kW or more of connected compressor power. In Germany's day-ahead electricity market (EPEX SPOT), prices can vary from negative values (≈−15 €/MWh) during periods of high renewable generation to several hundred euros per megawatt-hour during scarcity events.

Traditional thermostatic control cycles compressors to maintain a fixed internal setpoint temperature. This approach is **price-agnostic**: it runs cooling at the same temperature threshold regardless of whether electricity costs 10 €/MWh or 200 €/MWh.

### 2.2 The Opportunity

Refrigerated spaces are thermally sluggish. A well-insulated cold store with 200 tonnes of frozen product has a thermal time constant on the order of tens of hours. This means the facility can safely tolerate compressor downtimes of several hours — the stored mass acts as a thermal battery. When electricity is cheap, the system can pre-cool the space below its normal setpoint (thermal pre-charging). When electricity is expensive, it can let the temperature drift upward within safe bounds (load shedding). The thermal inertia provides a time buffer that can be monetised through electricity price arbitrage.

### 2.3 Challenges

Realising this arbitrage requires solving several interlocking problems:

1. **Predictive planning**: When is the cheapest window to pre-cool over the next 12 hours?
2. **Constraint enforcement**: Temperature must never exceed the product's safe upper limit (time-to-failure tracking) or drop below the compressor's minimum setpoint.
3. **Compressor protection**: Industrial compressors cannot short-cycle — they require minimum runtimes and off-times to prevent oil return failure and motor burnout.
4. **Grid stability participation**: The facility can provide frequency response services by modulating compressor load in reaction to grid imbalances.
5. **Parameter uncertainty**: Thermal capacitance changes with product loading, door openings, and defrost cycles; the system must adapt online.

---

## 3. System Architecture

The system is implemented as a modular, event-driven application with five principal subsystems:

```
┌──────────────────────────────────────────────────────────────┐
│                    DATA INGESTION LAYER                      │ 
│   ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │ 
│   │ Open-Meteo  │  │ Energy-Charts│  │ SMARD (Fallback)   │  │ 
│   │ (Weather)   │  │ (Prices)     │  │ (Prices)           │  │
│   └──────┬──────┘  └──────┬───────┘  └─────────┬──────────┘  │
│          └────────────────┼────────────────────┘             │
│                     ┌─────▼──────┐                           │
│                     │ JSON Cache │                           │
│                     └─────┬──────┘                           │
└───────────────────────────┼──────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────┐
│                   SIMULATION ENGINE                          │
│  ┌────────────────────────────────────────────────────────┐  │
│  │    2025 NRW Dataset (8,760 hourly data points)         │  │
│  │  ┌──────────┐  ┌───────────┐  ┌───────────────────┐    │  │
│  │  │Ambient   │  │Grid Freq. │  │ EPEX Price Model  │    │  │
│  │  │Temp.     │  │Model      │  │ (Synthetic/Real)  │    │  │
│  │  └──────────┘  └───────────┘  └───────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────┬───────────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────┐
│                    CONTROL ENGINE                            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  5-Mode Priority Cascade:                              │  │
│  │  1. Emergency (time-to-failure critical)               │  │
│  │  2. Thermal Guard (integral temperature violation)     │  │
│  │  3. Grid Balancing (frequency response)                │  │
│  │  4. MPC Pre-Cool (model predictive arbitrage)          │  │
│  │  5. Price Arbitrage (threshold-based)                  │  │
│  │  │ Short-Cycling Governor (state machine)           │  │  │
│  │  │ Min on-time: 900s | Min off-time: 600s           │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────┬───────────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────┐
│                   THERMAL MODEL                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ • 1st-order RC thermal dynamics                        │  │
│  │ • Dynamic Carnot COP (varies with ΔT)                  │  │
│  │ • RLS parameter estimation (adaptive C_th)             │  │
│  │ • Time-to-failure with confidence bounds               │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────┬───────────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────┐
│                FINANCIAL TRACKING                            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ Baseline vs. Arbitrage: Energy (MWh) & Cost (€)        │  │
│  │ Runtime duty cycle savings projection                  │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 3.1 Core Modules

| Module | Responsibility |
|---|---|
| **Data Ingestion** | Fetches real weather data (Open-Meteo Archive API) and day-ahead electricity prices (energy-charts.info / SMARD). Caches results locally as JSON for offline operation. Falls back to a calibrated synthetic model when APIs are unavailable. |
| **Simulation Engine** | Steps through a full year of historical/synthetic data (8,760 hours of ambient temperature, solar irradiance, grid frequency, and electricity price). Operates at accelerated time (1 real second = 1 simulated hour). |
| **Control Engine** | Implements the five-mode priority cascade. Receives sensor readings, grid frequency, and electricity price. Produces compressor on/off decisions, setpoint adjustments, and MPC strategy selections. |
| **Thermal Model** | Maintains a first-order equivalent thermal parameter (ETP) model of the refrigerated space. Computes dynamic Carnot COP, performs Recursive Least Squares (RLS) adaptation of thermal capacitance, and calculates time-to-failure (TTF) with uncertainty bounds. |
| **Financial Tracker** | Continuously compares actual energy consumption and cost against a baseline thermostat scenario. Reports savings percentages and annualised projections. |
| **HTTP API** | Axum-based REST server exposing live system state: `/status` returns current temperatures, compressor state, active controller mode, TTF, and price; `/price` allows manual price overrides. |
| **CSV Logger** | Writes time-stamped records of sensor readings and control decisions to a configurable CSV file for post-hoc analysis. |

---

## 4. Core Algorithms

### 4.1 Five-Mode Priority Cascade Controller

The control engine evaluates five operating modes in strict priority order. Higher-priority modes override lower-priority decisions:

#### Mode 1 — Emergency (Time-to-Failure Critical)

When the computed time-to-failure (TTF) — the time until internal temperature would reach the upper limit — falls below a configurable critical threshold (default: 900 seconds), the controller **immediately forces the compressor on** at the minimum setpoint. This mode **bypasses** the short-cycling governor, as product integrity takes precedence over compressor longevity.

```
if ttf < critical_threshold:
    compressor = ON
    setpoint = T_min
    mode = EMERGENCY
```

#### Mode 2 — Thermal Guard (Integral Temperature Violation)

The system maintains a **thermal integral**: a leaky accumulator of temperature excess above the normal setpoint. If the cumulative thermal stress exceeds a configurable maximum (default: 3,600 °C·s), the controller forces cooling at the normal setpoint to discharge the thermal debt. This prevents sustained temperature excursions that could compromise product quality.

```
if thermal_integral > max_integral:
    compressor = ON
    setpoint = T_normal
    mode = THERMAL_GUARD
```

#### Mode 3 — Grid Balancing (Frequency Response)

When grid frequency deviates beyond a configurable deadband (±0.1 Hz default), the facility participates in **demand-side frequency response**. Under-frequency (grid deficit) triggers compressor turn-on to increase load; over-frequency (grid surplus) triggers compressor turn-off to decrease load. The response is instantaneous within the control loop. This mode enables the facility to generate revenue through ancillary services markets.

```
if grid_deviation > +deadband:
    compressor = OFF   // shed load for over-frequency
    setpoint = T_max
    mode = GRID_BALANCING
elif grid_deviation < -deadband:
    compressor = ON    // add load for under-frequency
    setpoint = T_min
    mode = GRID_BALANCING
```

#### Mode 4 — MPC Pre-Cool (Model Predictive Arbitrage)

This is the primary economic optimisation mode. The controller looks ahead 6–12 hours in the price and weather forecast to find the cheapest contiguous window for pre-cooling.

The MPC solves the following optimisation:

```
Minimise J = Σ price(t) · power(t) · dt
Subject to:
    - T_min ≤ T(t) ≤ T_max              (temperature bounds)
    - T(t+1) = f(T(t), compressor(t))   (thermal dynamics)
    - compressor runtime constraints     (governor)
```

For each candidate start delay in the horizon, the MPC simulates the thermal trajectory and accumulates energy cost. It selects the window with the lowest predicted cost. The confidence level is computed as the ratio between the best and worst-case costs in the search space.

```
for start_delay in 0..horizon:
    simulate thermal path with cooling starting at start_delay
    compute total_cost = Σ energy_cost(h)
    if path remains within bounds:
        track best_cost and worst_cost
confidence = 1 - best_cost / worst_cost
```

When confidence exceeds 20% and the optimal window is imminent (<2 hours away), the controller pre-cools to the minimum setpoint, effectively charging the thermal battery at low cost. Otherwise, it operates with a wider (less energy-intensive) setpoint band to let the temperature float until the cheap window arrives.

#### Mode 5 — Price Arbitrage (Threshold-Based)

If MPC is inactive (insufficient forecast data or low confidence), the controller falls back to a rule-based price arbitrage:

| Price Condition | Action |
|---|---|
| < 30 €/MWh | Compressor ON → pre-cool to T_min (charge thermal battery) |
| 30–80 €/MWh | Normal thermostat operation at setpoint |
| > 80 €/MWh | Compressor OFF → shed load, allow drift to T_max (discharge thermal battery) |

### 4.2 Short-Cycling Governor

Industrial compressors require minimum on-time and off-time to maintain proper oil return and prevent motor overheating. A dedicated state machine enforces these constraints:

- **Minimum runtime**: 900 seconds (15 minutes) — the compressor must remain on for at least this duration once started.
- **Minimum off-time**: 600 seconds (10 minutes) — the compressor must remain off for at least this duration once stopped.

The governor tracks switching transitions and locks the compressor state until the minimum duration elapses. Only Emergency Mode (Mode 1) can override this lock. All timing uses the **simulated clock** to ensure correct behaviour at accelerated simulation speeds.

### 4.3 Dynamic Carnot COP Model

The system does not assume a fixed coefficient of performance (COP). Instead, it models COP dynamically based on the Carnot cycle:

```
COP_carnot = T_cold / (T_warm - T_cold)      (temperatures in Kelvin)
COP_real = η_carnot · COP_carnot
```

Where:
- `T_cold` = internal temperature (e.g., 253 K at −20°C)
- `T_warm` = ambient temperature (e.g., 308 K at 35°C)
- `η_carnot` = Carnot efficiency factor (default: 0.6, representing a realistic refrigeration system)

This dynamically adjusts the effective cooling power: when it is hot outside and the room is very cold, COP drops and cooling becomes less efficient. When the temperature delta is small, COP rises. The model also uses a midpoint approximation for large time steps to improve numerical accuracy.

### 4.4 Recursive Least Squares (RLS) Parameter Estimation

The thermal capacitance `C_th` of a cold store is not constant — it varies with product loading, moisture content, door activity, and defrost cycles. The system employs an online Recursive Least Squares estimator that watches the **temperature decay curve when the compressor is off** to update its estimate of `C_th`.

The RLS identifies the thermal time constant `τ = R_th · C_th` from the observed exponential temperature relaxation:

```
y = ln((T_after - T_ambient) / (T_before - T_ambient))
φ = -dt / R_th
θ = 1 / C_th
```

The Kalman gain update:

```
K = P · φ / (λ + φ² · P)
θ_new = θ_old + K · (y - φ · θ_old)
P = (1 - K · φ) · P / λ
```

Where `λ` (forgetting factor, default 0.98) controls how quickly old observations are discounted. The estimated `C_th` is then used in MPC lookahead simulations and time-to-failure calculations, making the system adaptive to changing facility conditions.

### 4.5 Time-to-Failure (TTF) with Confidence Bounds

TTF answers the question: **"If the compressor stays off, how long until the internal temperature reaches the upper limit?"** It is computed from the current thermal state and ambient temperature:

```
TTF = τ · ln((T_limit - T_ambient) / (T_current - T_ambient))
```

The RLS parameter uncertainty is propagated into confidence bounds:

- **Lower bound** (pessimistic): uses the lower estimate of `C_th` → faster warmup → shorter TTF
- **Upper bound** (optimistic): uses the upper estimate of `C_th` → slower warmup → longer TTF
- **Base estimate**: uses the current estimated `C_th`

These bounds allow operators to assess risk in the arbitrage strategy — a narrow confidence band means the thermal behaviour is well-understood; a wide band signals uncertainty that warrants a more conservative approach.

---

## 5. Data Sources and Models

### 5.1 Weather Data (Open-Meteo Archive API)

The system fetches hourly historical weather data for the NRW region of Germany (latitude 51.45°N, longitude 7.01°E — Essen/Düsseldorf/Köln area):

- **Temperature**: 2-metre air temperature (°C)
- **Solar irradiance**: Shortwave radiation (GHI, W/m²)
- **Cloud cover**: Percentage (%)
- **Period**: Full year 2025 (8,760 hours)

Open-Meteo is a free, open-source API requiring no authentication, with a 10,000 requests/day limit — sufficient for periodic updates.

### 5.2 Electricity Prices (energy-charts.info / SMARD)

Day-ahead electricity prices for Germany are sourced from:

1. **Primary**: energy-charts.info API (Fraunhofer ISE) — returns day-ahead auction prices for the German market in €/MWh.
2. **Fallback**: SMARD direct data (German Federal Network Agency) — provides the same underlying market data.

Prices reflect the EPEX SPOT day-ahead auction and can range from negative values (renewable oversupply) to several hundred euros per megawatt-hour (scarcity events).

### 5.3 Grid Frequency Model

Grid frequency data is modelled with realistic statistical properties matching the Continental Europe synchronous area:

- **Mean**: 50.0 Hz
- **Standard deviation**: ≈0.05 Hz in normal operation
- **Autocorrelation**: Strong (τ ≈ 15 minutes)
- **Range**: 49.8–50.2 Hz (99.7% of time)
- **Events**: Occasional imbalance events (±0.1–0.2 Hz) from generation/demand mismatches
- **Solar ramp effects**: Frequency excursions during high-PV ramp periods (spring/autumn mornings)

### 5.4 Synthetic Fallback Model

When internet access is unavailable and no local cache exists, the system generates a fully synthetic dataset using:

- **Cubic Hermite spline interpolation** of DWD 1991–2020 climate normals for monthly temperatures
- **AR(1) autoregressive noise** for realistic day-to-day temperature persistence
- **ESRA clear-sky solar model** with Linke turbidity (TL = 3.5 for Central Europe)
- **3-state Markov chain** for cloud cover transitions (clear / partly cloudy / overcast)
- **EPEX SPOT price model** calibrated to 2024 German market data, incorporating time-of-day profiles, solar merit-order effects, "Dunkelflaute" winter spikes, temperature effects, weekend patterns, and stochastic volatility
- **Grid frequency model** with the statistical properties described above

---

## 6. Building Physics and Configuration

### 6.1 Thermal Parameter Computation

The system accepts a high-level building description and derives equivalent thermal parameters:

**Thermal Resistance `R_th`:**

R_th accounts for heat ingress through two parallel paths:
- **Structural conductance**: `U_value · surface_area`
- **Air exchange conductance**: `air_flow_rate · air_density · specific_heat_capacity`

```
R_th = 1 / (U·A + V·ACH/3600 · ρ·c_p)
```

**Thermal Capacitance `C_th`:**

C_th aggregates three heat storage contributions:
- **Product mass**: `mass · specific_heat` (dominant term, e.g., 200 tonnes of frozen food)
- **Air mass**: `volume · 1.2 · 1005` (negligible compared to product)
- **Structure**: estimated at 20% of product capacitance

**Time Constant:**
```
τ = R_th · C_th
```

For a typical medium cold store (1,000 m³, 200 tonnes product, U=0.25), the time constant is on the order of 20–40 hours — providing substantial flexibility for load shifting.

### 6.2 Baseline Duty Cycle

The financial comparison uses a physics-based baseline model. At steady state, the heat ingress into the cold store must be balanced by the cooling provided:

```
heat_leak = (T_ambient - T_setpoint) / R_th
cooling_provided = P_installed · COP · duty_cycle
duty_cycle = heat_leak / (P_installed · COP)
```

This yields a realistic duty cycle (typically 30–50% for moderate climates) that varies with ambient temperature — when it is cold outside, heat leak is minimal, and the compressor may run infrequently. This avoids the unrealistic assumption of fixed compressor runtime found in simpler models.

### 6.3 Pre-Configured Building Templates

The system ships with three building templates:

| Template | Volume | Product Mass | Cooling Power | Typical Use Case |
|---|---|---|---|---|
| Small Walk-In Cooler | 50 m³ | 5 tonnes | 5 kW | Restaurant / retail back-of-house |
| Medium Cold Storage (default) | 1,000 m³ | 200 tonnes | 50 kW | Regional distribution centre |
| Large Warehouse | 10,000 m³ | 2,000 tonnes | 200 kW | Major cold storage facility |

---

## 7. Financial Tracking

### 7.1 Baseline Comparison

The financial module continuously compares two scenarios in parallel:

**Baseline**: What the facility would have consumed under a traditional thermostat maintaining the normal setpoint. The compressor runs at the physics-based duty cycle (Section 6.2).

**Actual**: What is actually consumed under the arbitrage/control strategy. The compressor state from the control decision is applied directly.

### 7.2 Key Metrics

|        Metric         |                Formula                |               Interpretation               |
|-----------------------|---------------------------------------|--------------------------------------------|
| Energy Saved (MWh)    | `E_baseline − E_actual`               | Positive = net energy reduction            |
| Money Saved (€)       | `C_baseline − C_actual`               | Positive = cost reduction                  |
| Savings (%)           | `Money_Saved / C_baseline × 100`      | Percentage cost savings                    |
| Annualised Projection | `Money_Saved / Hours_Tracked × 8,760` | Extrapolated yearly savings                |
| Average Price Paid    | `C_actual / E_actual`                 | Effective electricity cost under arbitrage |

The system tracks these metrics over the entire operating period, providing a clear picture of the financial benefit of the arbitrage strategy.

## 8. Operational Interface

### 8.1 HTTP API

The system exposes a lightweight REST API for integration with building management systems, SCADA platforms, or custom dashboards:

**`GET /status`** — Returns a JSON payload with current operating state:

```json
{
  "unit_id": "nrw-store-001",
  "running": true,
  "current_temp_c": -22.3,
  "compressor_on": true,
  "active_controller": "MpcPreCool",
  "ttf_sec": 7200.0,
  "thermal_integral": 245.0,
  "grid_frequency_hz": 50.03,
  "price_eur_per_mwh": 12.50
}
```

**`POST /price`** — Allows manual price override for custom scenarios or integration with external price forecasting services:

```json
{
  "price_eur_per_mwh": 85.00
}
```

### 8.2 Dashboard Display

During operation, the system renders a real-time terminal dashboard showing:

- Calendar date and season (for the current simulated time)
- Simulation progress (percentage of year completed)
- Ambient temperature with visual thermometer bar
- Storage temperature relative to operating bounds
- Compressor state, power consumption, and active control mode
- Grid frequency with visual deviation indicator
- Current electricity price
- Financial summary table (energy, costs, savings)
- Current controller action explanation

### 8.3 CSV Logging

All sensor readings and control decisions are logged to a configurable CSV file for post-hoc analysis. Each record includes: timestamp, internal/ambient temperatures, compressor state, power draw, grid frequency, controller mode, setpoint, TTF, thermal integral, and price. This data enables operators to validate the system's decisions, optimise parameters, and generate compliance reports.

---

## 9. Configuration Reference

The system is configured through a TOML configuration file with the following sections:

| Section | Key Parameters | Description |
|---|---|---|
| `[system]` | `unit_id`, `loop_interval_sec` | System identity and control loop timing |
| `[thermal]` | `r_th`, `c_th`, `t_min`, `t_max`, `t_setpoint_*` | Thermal model parameters and temperature bounds |
| `[control]` | `thermal_integral_max`, `ttf_critical_sec`, `droop_gain_k`, `min_runtime_sec`, `min_off_time_sec`, `grid_balancing_deadband_hz`, `mpc_cheap_threshold_eur_per_mwh` | Controller tuning parameters |
| `[logging]` | `csv_path`, `interval_sec` | Output logging configuration |
| `[api]` | `bind_address` | HTTP API listen address |
| `[sensor]` | `simulate`, `ambient_temp_c`, `cooling_power_w` | Sensor configuration (simulated or live) |
| `[building]` | `volume_m3`, `surface_area_m2`, `u_value`, `product_mass_kg`, `cop`, `installed_cooling_power_w`, `air_ach_per_hour` | Physical building parameters |

---

## 10. Use Cases

### 10.1 Electricity Price Arbitrage

The primary use case. The facility shifts compressor runtime from expensive peak hours to cheaper off-peak hours or periods of renewable oversupply (negative prices). The MPC ensures pre-cooling happens at the optimal time within the forecast horizon. Typical savings of 15–30% on electricity costs have been observed in simulation, depending on market conditions and facility characteristics.

### 10.2 Grid Balancing Services

By responding to frequency deviations, the facility can participate in:
- **Frequency Containment Reserve (FCR)**: Instantaneous response to grid imbalances
- **Automatic Frequency Restoration Reserve (aFRR)**: Sustained response over 5–15 minute periods

The frequency response mode operates at higher priority than arbitrage, ensuring grid stability obligations are met before pursuing economic optimisation.

### 10.3 Peak Load Management

In addition to price arbitrage, the system inherently reduces peak demand by avoiding compressor operation during high-price periods. This has secondary benefits for grid congestion management and can reduce network charges for facilities on demand-based tariffs.

### 10.4 Thermal Performance Monitoring

The RLS parameter estimation provides continuous monitoring of the facility's thermal characteristics. A gradual decrease in estimated `C_th` may indicate reduced product loading or increased air infiltration (e.g., worn door seals). An unexpected increase in `R_th` (reduced thermal resistance) can signal insulation degradation. This transforms the system into a diagnostic tool for cold store maintenance.

---

## 11. Safety and Constraint Enforcement

The system is designed with multiple layers of safety to ensure product integrity is never compromised in pursuit of cost savings:

1. **Emergency override** (highest priority): If TTF drops below the critical threshold, the compressor forces on unconditionally.
2. **Thermal integral guard**: Prevents sustained temperature excursions that could cumulatively damage product quality.
3. **Temperature bounds**: The controller never allows internal temperature to exceed the absolute `t_max` or drop below `t_min`.
4. **Governor bypass insurance**: Only Emergency Mode can override the short-cycling governor, ensuring compressor protection is maintained in normal operation.
5. **Confidence-based MPC**: If the MPC's confidence is low (<20%), the system reverts to simpler threshold-based control rather than acting on an uncertain forecast.

---

## 12. Summary

Ambient Thermal Arbitrage transforms a conventional industrial refrigeration system — traditionally a fixed electricity load — into a flexible, price-responsive grid asset. By exploiting the thermal inertia inherent in cold storage facilities, the system:

- **Reduces electricity costs** through intelligent load shifting and price arbitrage
- **Generates additional revenue streams** through grid balancing services
- **Provides thermal performance diagnostics** via adaptive parameter estimation
- **Maintains full product safety** through layered constraint enforcement
- **Integrates with existing infrastructure** via standard REST APIs and CSV logging

The modular architecture, adaptive algorithms, and physics-based modelling make the system suitable for a wide range of cold storage facilities — from small walk-in coolers to large-scale warehouse refrigeration.

---

*Ambient Thermal Arbitrage — Turning Cold Storage into a Grid Asset*

*For further information, configuration guidance, or integration inquiries, please refer to the project documentation or contact the development team.*