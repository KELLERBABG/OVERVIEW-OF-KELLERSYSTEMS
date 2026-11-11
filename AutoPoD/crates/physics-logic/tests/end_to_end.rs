use physics_logic::conditions::{evaluate_combination, evaluate_policy};
use pod_core::policy::{CombinationLogic, DamagePolicy, DamageThreshold, ThresholdParameter};
use pod_core::sensor::{SensorReading, SensorType};
use uuid::Uuid;

fn reading(sensor_type: SensorType, value: f64, ts: i64) -> SensorReading {
    SensorReading {
        sensor_id: Uuid::nil(),
        sensor_type,
        timestamp: ts,
        value,
        unit: "g".into(),
    }
}

#[test]
fn test_end_to_end_collision_detected() {
    let policy = DamagePolicy {
        name: "frontal-collision".into(),
        thresholds: vec![
            DamageThreshold {
                name: "impact_g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(8.0),
                duration_ms: None,
            },
            DamageThreshold {
                name: "rapid_decel".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::RateOfChange {
                    max_delta: 5.0,
                    over_duration_ms: 500,
                },
                duration_ms: None,
            },
        ],
        combination: CombinationLogic::All,
    };

    let readings = vec![
        reading(SensorType::Accelerometer, 0.3, 0),
        reading(SensorType::Accelerometer, 0.4, 100),
        reading(SensorType::Accelerometer, 0.3, 200),
        reading(SensorType::Accelerometer, 2.0, 300),
        reading(SensorType::Accelerometer, 6.0, 400),
        reading(SensorType::Accelerometer, 10.0, 500),
        reading(SensorType::Accelerometer, 14.0, 600),
        reading(SensorType::Accelerometer, 12.0, 700),
        reading(SensorType::Accelerometer, 8.0, 800),
        reading(SensorType::Accelerometer, 4.0, 900),
    ];

    let conditions = evaluate_policy(&policy, &readings);
    assert_eq!(conditions.len(), 2);

    let impact = conditions.iter().find(|c| c.name == "impact_g").unwrap();
    let decel = conditions.iter().find(|c| c.name == "rapid_decel").unwrap();
    assert!(impact.satisfied, "impact_g should be satisfied (max: 14.0 > 8.0)");
    assert!(decel.satisfied, "rapid_decel should detect the rate of change");

    let approved = evaluate_combination(&conditions, &CombinationLogic::All);
    assert!(approved, "collision should be approved");
}

#[test]
fn test_end_to_end_normal_driving_rejected() {
    let policy = DamagePolicy {
        name: "collision-detect".into(),
        thresholds: vec![DamageThreshold {
            name: "impact_g".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::GreaterThan(8.0),
            duration_ms: None,
        }],
        combination: CombinationLogic::Any,
    };

    let readings: Vec<_> = (0..50)
        .map(|i| reading(SensorType::Accelerometer, 0.3 + (i as f64 * 0.01).sin() * 0.2, i * 100))
        .collect();

    let conditions = evaluate_policy(&policy, &readings);
    let approved = evaluate_combination(&conditions, &CombinationLogic::Any);
    assert!(!approved, "normal driving should not trigger collision");
}

#[test]
fn test_end_to_end_sensor_integrity_fraud() {
    let policy = DamagePolicy {
        name: "fraud-detect".into(),
        thresholds: vec![DamageThreshold {
            name: "impossible_g".into(),
            sensor_type: SensorType::Accelerometer,
            parameter: ThresholdParameter::GreaterThan(50.0),
            duration_ms: None,
        }],
        combination: CombinationLogic::Any,
    };

    let readings = vec![
        reading(SensorType::Accelerometer, 0.3, 0),
        reading(SensorType::Accelerometer, 0.4, 100),
        reading(SensorType::Accelerometer, 99.9, 200),
        reading(SensorType::Accelerometer, 0.3, 300),
    ];

    let conditions = evaluate_policy(&policy, &readings);
    let approved = evaluate_combination(&conditions, &CombinationLogic::Any);
    assert!(approved, "fraud spike should still satisfy the threshold");
}

#[test]
fn test_end_to_end_policy_from_json() {
    let policy_json = r#"{
        "name": "test-policy",
        "thresholds": [{
            "name": "speed",
            "sensor_type": "Speedometer",
            "parameter": { "LessThan": 30.0 },
            "duration_ms": null
        }],
        "combination": "Any"
    }"#;

    let policy: DamagePolicy = serde_json::from_str(policy_json).unwrap();
    let readings = vec![reading(SensorType::Speedometer, 25.0, 0)];

    let conditions = evaluate_policy(&policy, &readings);
    assert!(conditions[0].satisfied);
}
