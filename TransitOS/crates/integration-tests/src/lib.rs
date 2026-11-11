#[cfg(test)]
mod integration {
    use transit_abstract_interpretation::{
        domain::Interval,
        symbolic::{var, SymbolicExpr},
        AbstractInterpreter,
    };
    use transit_deployment_gate::{DeploymentGate, GateVerdict};
    use transit_dynamics_simulator::{
        scenarios::{emergency_brake, lane_change},
        simulator::Simulator,
    };
    use transit_gas_brake_analysis::{
        analysis::analyze_emergency_brake,
        energy::{verify_energy_budget, verify_max_brake_feasibility},
    };
    use transit_path_constraints::maneuvers::emergency_brake_constraint;
    use transit_smt_solver::query::{CollisionQuery, Obstacle};
    use transit_smt_solver::SmtSolver;
    use transit_spec_generator::generate::{infer_ltl_spec, infer_spec_from_trajectory};
    use transit_state_machine::{
        machine::StateMachine,
        params::VehicleParams,
        state::{ActuatorState, Dynamics, Pose, VehicleState},
        transitions::StateTransition,
    };
    use transit_state_space::{
        explorer::{explore_region, ExplorationStrategy, ExplorerConfig},
        region::StateRegion,
    };
    use transit_taint_analysis::{sensors::SensorType, taint::TaintedValue};
    use transit_verification_invariants::{
        invariants::{default_safety_invariants, BrakingDistanceInvariant, SafetyInvariant},
        monitor::RuntimeMonitor,
        proof::{verify_all, ProofObligation},
    };

    #[test]
    fn test_vehicle_state_creation() {
        let state = VehicleState {
            pose: Pose {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
            },
            dynamics: Dynamics {
                velocity: 10.0,
                lateral_velocity: 0.0,
                acceleration: 0.0,
                yaw_rate: 0.0,
                slip_angle: 0.0,
                side_slip: 0.0,
            },
            articulation: Default::default(),
            actuators: ActuatorState {
                steering_angle: 0.0,
                brake_pressure: 0.0,
                throttle: 0.0,
                brake_commanded: 0.0,
                brake_applied: 0.0,
            },
            tires: Default::default(),
            road: Default::default(),
        };
        assert!(!state.is_stationary());
        assert!((state.speed() - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_state_machine_transition() {
        let scenario = emergency_brake(20.0, Default::default());
        let params = VehicleParams::default();
        let mut sm = StateMachine::new(scenario.initial_state, params);
        let input = StateTransition {
            dt: 0.1,
            steering_input: 0.0,
            throttle_input: 0.0,
            brake_input: 0.5,
        };
        sm.step(&input);
        assert_eq!(sm.history.len(), 2);
    }

    #[test]
    fn test_simulator_step() {
        let scenario = lane_change(0.0, 0.0, 15.0, Default::default());
        let mut sim = Simulator::new(scenario.initial_state, VehicleParams::default());
        sim.step(0.0, 0.3, 0.0);
        assert!(sim.time > 0.0);
    }

    #[test]
    fn test_abstract_interval_arithmetic() {
        let expr = SymbolicExpr::Add(Box::new(var("v")), Box::new(SymbolicExpr::Constant(5.0)));
        let mut env = std::collections::HashMap::new();
        env.insert("v".into(), Interval::new(0.0, 10.0));
        let result = AbstractInterpreter::eval(&expr, &env);
        assert!((result.lo - 5.0).abs() < 1e-6);
        assert!((result.hi - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_emergency_brake_analysis() {
        let scenario = emergency_brake(30.0, Default::default());
        let state = scenario.initial_state;
        let result = analyze_emergency_brake(&state, 50.0, &Default::default());
        assert!(!result.feasible || result.safety_margin >= 0.0);
    }

    #[test]
    fn test_energy_verification() {
        let scenario = emergency_brake(10.0, Default::default());
        assert!(verify_energy_budget(
            &scenario.initial_state,
            1500.0,
            12000.0,
            50.0
        ));
    }

    #[test]
    fn test_max_brake_feasibility() {
        let scenario = emergency_brake(30.0, Default::default());
        let dist = verify_max_brake_feasibility(&scenario.initial_state, &Default::default());
        assert!(dist > 0.0);
    }

    #[test]
    fn test_collision_detection() {
        let obstacles = vec![Obstacle {
            position: (50.0, 0.0),
            velocity: (0.0, 0.0),
            half_length: 2.0,
            half_width: 1.0,
        }];
        let query = CollisionQuery::new((0.0, 0.0), 10.0, 0.0, obstacles, 3.0);
        let solver = SmtSolver::new();
        let result = solver.check_collision(&query);
        assert!(!matches!(
            result,
            transit_smt_solver::query::SolverResult::Sat
        ));
    }

    #[test]
    fn test_invariant_checking() {
        let scenario = emergency_brake(20.0, Default::default());
        let inv: Box<dyn SafetyInvariant> = Box::new(BrakingDistanceInvariant {
            safety_gap: 100.0,
            max_deceleration: 8.0,
        });
        assert!(inv.check(&scenario.initial_state, &VehicleParams::default()));
    }

    #[test]
    fn test_default_invariants() {
        let scenario = emergency_brake(15.0, Default::default());
        let invariants = default_safety_invariants(Default::default(), 50.0, 100.0);
        let results = verify_all(
            &invariants,
            &scenario.initial_state,
            &VehicleParams::default(),
        );
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_tainted_actuator_blocked() {
        let tainted_cmd: TaintedValue<()> =
            TaintedValue::tainted((), SensorType::Camera, "camera_01");
        let query = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![], 10.0);
        let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
        let decision = DeploymentGate::evaluate(
            &VehicleState::default(),
            &VehicleParams::default(),
            &inv,
            &tainted_cmd,
            &query,
        );
        assert!(matches!(decision.verdict, GateVerdict::Block));
    }

    #[test]
    fn test_runtime_monitor() {
        let scenario = emergency_brake(20.0, Default::default());
        let invariants = default_safety_invariants(Default::default(), 50.0, 100.0);
        let mut monitor = RuntimeMonitor::new(invariants);
        let all_safe = monitor.check(&scenario.initial_state, 0.0, &VehicleParams::default());
        assert!(all_safe);
        assert!(monitor.get_violations().is_empty());
    }

    #[test]
    fn test_state_space_exploration() {
        let region = StateRegion::nominal_driving();
        let config = ExplorerConfig {
            strategy: ExplorationStrategy::Random { samples: 50 },
            ..Default::default()
        };
        let points = explore_region(&region, &config);
        assert!(!points.is_empty());
        assert_eq!(points.len(), 50);
    }

    #[test]
    fn test_path_constraints() {
        let set = emergency_brake_constraint(50.0);
        assert!(!set.is_empty());
    }

    #[test]
    fn test_bicycle_dynamics() {
        let scenario = emergency_brake(20.0, Default::default());
        let params = VehicleParams::default();
        let mut sm = StateMachine::new(scenario.initial_state, params);
        let input = StateTransition {
            dt: 0.01,
            steering_input: 0.1,
            throttle_input: 0.0,
            brake_input: 0.3,
        };
        sm.step(&input);
        assert!(sm.current.dynamics.yaw_rate.abs() > 0.0 || sm.current.dynamics.velocity < 20.0);
    }

    #[test]
    fn test_proof_obligation() {
        let scenario = emergency_brake(30.0, Default::default());
        let inv: Box<dyn SafetyInvariant> = Box::new(BrakingDistanceInvariant {
            safety_gap: 200.0,
            max_deceleration: 8.0,
        });
        let obligation =
            ProofObligation::new(inv, scenario.initial_state, VehicleParams::default());
        assert!(obligation.verify());
    }

    #[test]
    fn test_scenario_creation() {
        let brake_scenario = emergency_brake(20.0, Default::default());
        assert!(brake_scenario.initial_state.dynamics.velocity > 0.0);

        let lane_scenario = lane_change(0.0, 0.0, 10.0, Default::default());
        assert!(lane_scenario.initial_state.dynamics.velocity > 0.0);
    }

    #[test]
    fn test_abstract_safety_condition() {
        use transit_abstract_interpretation::analysis::verify_braking_safety;
        assert!(!verify_braking_safety(
            Interval::new(30.0, 30.0),
            Interval::new(0.3, 0.3),
            20.0
        ));
        assert!(verify_braking_safety(
            Interval::new(5.0, 5.0),
            Interval::new(0.8, 0.8),
            20.0
        ));
    }

    #[test]
    fn test_spec_generator_infers_specs_from_trajectory() {
        let scenario = emergency_brake(20.0, Default::default());
        let mut sm = StateMachine::new(scenario.initial_state, VehicleParams::default());
        for _ in 0..10 {
            sm.step(&StateTransition {
                dt: 0.05,
                steering_input: 0.0,
                throttle_input: 0.0,
                brake_input: 0.4,
            });
        }
        let trajectory = sm.history.clone();
        let spec = infer_spec_from_trajectory(&trajectory);
        assert!(spec.max_velocity >= 0.0);
        let ltls = infer_ltl_spec(&trajectory);
        assert!(!ltls.is_empty());
    }

    mod phase3 {
        use transit_carla_integration::{
            bridge::CarlaBridge, scenario_runner::ScenarioRunner, sensor::SensorFusion,
        };
        use transit_deployment_gate::{
            audit::AuditLog,
            gate::Interceptor,
            pipeline::VerificationPipeline,
            policy::GatePolicy,
            verdict::{GateDecision, GateVerdict},
        };
        use transit_monitoring::{
            dashboard::Dashboard,
            history::ViolationHistory,
            integrity::{IntegrityStatus, SensorIntegrityReport},
            metrics::SafetyMetrics,
        };
        use transit_real_time::{
            latency::{LatencyBudget, LatencyMonitor},
            scheduling::{SafetyVerificationTask, TaskSpec},
            wcet::{VerificationTiming, WcetAnalyzer},
        };
        use transit_ros2_integration::{
            bridge::Ros2Bridge,
            messages::{ActuatorCommandMsg, VehicleStateMsg},
            node::SafetyNode,
        };
        use transit_smt_solver::query::CollisionQuery;
        use transit_state_machine::params::VehicleParams;
        use transit_state_machine::state::VehicleState;
        use transit_taint_analysis::{
            sensors::{SensorSource, SensorType},
            taint::TaintedValue,
        };
        use transit_verification_invariants::invariants::{SafetyInvariant, SpeedLimitInvariant};

        #[test]
        fn test_interceptor_allows_safe() {
            let mut interceptor = Interceptor::new();
            let cmd = TaintedValue::clean(());
            let query = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![], 10.0);
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let decision = interceptor.intercept(
                &VehicleState::default(),
                &VehicleParams::default(),
                &cmd,
                &query,
                &inv,
            );
            assert!(matches!(decision.verdict, GateVerdict::Allow));
        }

        #[test]
        fn test_interceptor_blocks_tainted() {
            let mut interceptor = Interceptor::new();
            let cmd = TaintedValue::tainted((), SensorType::Camera, "cam_0");
            let query = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![], 10.0);
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let decision = interceptor.intercept(
                &VehicleState::default(),
                &VehicleParams::default(),
                &cmd,
                &query,
                &inv,
            );
            assert!(matches!(decision.verdict, GateVerdict::Block));
        }

        #[test]
        fn test_verification_pipeline() {
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let mut pipeline = VerificationPipeline::new(GatePolicy::Normal, inv);
            let cmd = TaintedValue::clean(());
            let query = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![], 10.0);
            let decision = pipeline.verify(
                &VehicleState::default(),
                &VehicleParams::default(),
                &cmd,
                &query,
            );
            assert!(matches!(decision.verdict, GateVerdict::Allow));
        }

        #[test]
        fn test_pipeline_block_rate() {
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let mut pipeline = VerificationPipeline::new(GatePolicy::Strict, inv);
            let cmd = TaintedValue::clean(());
            let query = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![], 10.0);
            pipeline.verify(
                &VehicleState::default(),
                &VehicleParams::default(),
                &cmd,
                &query,
            );
            assert!(pipeline.block_rate() >= 0.0);
        }

        #[test]
        fn test_audit_log_records_blocks() {
            let mut audit = AuditLog::new();
            let d1 = GateDecision::block(1, "test");
            let d2 = GateDecision::allow(2);
            audit.record(d1, 100.0, vec!["check".into()]);
            audit.record(d2, 50.0, vec!["check".into()]);
            assert_eq!(audit.entries().len(), 2);
            assert_eq!(audit.blocks().len(), 1);
        }

        #[test]
        fn test_ros2_safety_node() {
            let node = SafetyNode::new("test_node");
            let decision = GateDecision::allow(1);
            let msg = node.publish_verdict(&decision);
            assert!(msg.allowed);
        }

        #[test]
        fn test_ros2_bridge() {
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let pipeline = VerificationPipeline::new(GatePolicy::Normal, inv);
            let mut bridge = Ros2Bridge::new(pipeline);
            let cmd = ActuatorCommandMsg {
                steering: 0.0,
                throttle: 0.0,
                brake: 0.0,
                source_id: "test".into(),
                sources: Vec::new(),
            };
            let ego = VehicleStateMsg {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                velocity: 0.0,
                yaw_rate: 0.0,
                steering_angle: 0.0,
                params: VehicleParams::default(),
            };
            let (verdict, _) = bridge.on_actuator_command(&cmd, &ego, &[]);
            assert!(verdict.allowed);
        }

        #[test]
        fn test_ros2_bridge_blocks_speeding_ego() {
            let inv: Vec<Box<dyn SafetyInvariant>> =
                vec![Box::new(SpeedLimitInvariant { max_speed: 50.0 })];
            let pipeline = VerificationPipeline::new(GatePolicy::Normal, inv);
            let mut bridge = Ros2Bridge::new(pipeline);
            let cmd = ActuatorCommandMsg {
                steering: 0.0,
                throttle: 1.0,
                brake: 0.0,
                source_id: "test".into(),
                sources: Vec::new(),
            };
            let ego = VehicleStateMsg {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                velocity: 60.0,
                yaw_rate: 0.0,
                steering_angle: 0.0,
                params: VehicleParams::default(),
            };
            let (verdict, _) = bridge.on_actuator_command(&cmd, &ego, &[]);
            assert!(!verdict.allowed);
        }

        #[test]
        fn test_ros2_bridge_blocks_low_trust_sources() {
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let pipeline = VerificationPipeline::new(GatePolicy::Normal, inv);
            let mut bridge = Ros2Bridge::new(pipeline);
            let cmd = ActuatorCommandMsg {
                steering: 0.0,
                throttle: 0.0,
                brake: 0.0,
                source_id: "test".into(),
                sources: vec![SensorSource {
                    sensor: SensorType::Radar,
                    id: "radar_0".into(),
                }],
            };
            let ego = VehicleStateMsg {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                velocity: 10.0,
                yaw_rate: 0.0,
                steering_angle: 0.0,
                params: VehicleParams::default(),
            };
            let (verdict, _) = bridge.on_actuator_command(&cmd, &ego, &[]);
            assert!(!verdict.allowed);
        }

        #[test]
        fn test_carla_scenario_runner() {
            let inv: Vec<Box<dyn SafetyInvariant>> = Vec::new();
            let pipeline = VerificationPipeline::new(GatePolicy::Normal, inv);
            let bridge = Ros2Bridge::new(pipeline);
            let mut runner = ScenarioRunner::new(bridge);
            let scenario =
                transit_dynamics_simulator::scenarios::emergency_brake(10.0, Default::default());
            let run = runner.run_scenario(&scenario);
            assert!(run.all_verdicts_allowed());
        }

        #[test]
        fn test_sensor_fusion() {
            let fusion = SensorFusion::new();
            let obs = vec![];
            let fused = fusion.fuse_obstacles(&obs, &obs, &obs);
            assert!(fused.is_empty());
        }

        #[test]
        fn test_latency_budget() {
            let budget = LatencyBudget::default();
            assert!(budget.meets_deadline(400.0));
            assert!(budget.total_max_latency() > 0.0);
        }

        #[test]
        fn test_latency_monitor() {
            let mut monitor = LatencyMonitor::new();
            monitor.record("verify", std::time::Duration::from_micros(150));
            assert!(monitor.max_latency("verify").as_micros() >= 150);
        }

        #[test]
        fn test_wcet_analyzer() {
            let mut analyzer = WcetAnalyzer::new();
            analyzer.record("smt", std::time::Duration::from_micros(200));
            assert_eq!(analyzer.wcet("smt").as_micros(), 200);
        }

        #[test]
        fn test_verification_timing() {
            let timing = VerificationTiming::default();
            assert!(timing.total_us() > 0.0);
        }

        #[test]
        fn test_safety_verification_task() {
            let task = SafetyVerificationTask::new();
            assert!(task.meets_control_deadline(100.0, 50.0));
        }

        #[test]
        fn test_task_schedulability() {
            let task = TaskSpec::new("test", 1000.0, 200.0);
            assert!(task.is_schedulable_rm(task.utilization(), 1));
        }

        #[test]
        fn test_dashboard_records_checks() {
            let mut dash = Dashboard::new();
            dash.record_check(true, 100.0, vec![]);
            assert_eq!(dash.metrics.total_checks, 1);
        }

        #[test]
        fn test_dashboard_snapshot() {
            let dash = Dashboard::new();
            let snap = dash.snapshot();
            assert_eq!(snap.metrics.total_checks, 0);
        }

        #[test]
        fn test_violation_history() {
            let mut hist = ViolationHistory::new();
            hist.record_violation(vec!["speed".into()], 200.0);
            assert_eq!(hist.count(), 1);
            assert!(hist.most_recent().is_some());
        }

        #[test]
        fn test_sensor_integrity() {
            let mut report = SensorIntegrityReport::new();
            assert_eq!(report.overall, IntegrityStatus::Healthy);
            report.mark_sensor_compromised("camera");
            assert_eq!(report.overall, IntegrityStatus::Compromised);
        }

        #[test]
        fn test_safety_metrics_status() {
            let mut metrics = SafetyMetrics::new();
            metrics.record_check(false, 100.0);
            metrics.record_check(false, 100.0);
            assert!(metrics.total_blocked > 0);
        }

        #[test]
        fn test_carla_bridge_translate() {
            let state = Default::default();
            let msg = CarlaBridge::translate_state_to_msg(&state, &VehicleParams::default());
            assert_eq!(msg.velocity, 0.0);
        }
    }
}
