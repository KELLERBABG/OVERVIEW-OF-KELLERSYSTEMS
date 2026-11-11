use muonscope_daq::barycenter::TriangularBarycenter;
use muonscope_daq::calibration::ChannelDeskewCalibrator;
use muonscope_daq::coordinate_frame::StationAttitude;
use muonscope_daq::flux_normalizer::FluxNormalizer;
use muonscope_daq::poca_core::{calculate_poca_intersection, LiveVoxelAccumulator};
use muonscope_daq::ring_buffer::SpscRingBuffer;
use muonscope_daq::track_kalman::KalmanTrackFitter;
use muonscope_daq::{CorrelatedEvent, PlaneHit, Point3, Vector3};

#[test]
fn test_ring_buffer_fifo() {
    let mut ring = SpscRingBuffer::<u32>::new(8);
    assert!(ring.is_empty());

    for i in 0..8 {
        assert!(ring.try_push(i * 10));
    }
    assert_eq!(ring.len(), 8);

    for i in 0..8 {
        assert_eq!(ring.try_pop(), Some(i * 10));
    }
    assert!(ring.is_empty());
}

#[test]
fn test_triangular_barycenter_interpolation() {
    let bary = TriangularBarycenter::new(20.0, 64);

    let pos_single = bary.calculate_hit_pos(32, 1000.0, None, None);
    assert!((pos_single - 10.0).abs() < 1e-3);

    let pos_shared = bary.calculate_hit_pos(32, 500.0, Some(33), Some(500.0));
    assert!((pos_shared - 20.0).abs() < 1e-3);
}

#[test]
fn test_kalman_track_fitting() {
    let mut fitter = KalmanTrackFitter::new();

    let hits = vec![
        PlaneHit { plane_id: 0, x_mm: 0.0, y_mm: 0.0, z_mm: 375.0, charge_adc: 1000, timestamp_ps: 100 },
        PlaneHit { plane_id: 1, x_mm: 0.0, y_mm: 0.0, z_mm: 365.0, charge_adc: 1000, timestamp_ps: 120 },
        PlaneHit { plane_id: 2, x_mm: 0.0, y_mm: 0.0, z_mm: 235.0, charge_adc: 1000, timestamp_ps: 200 },
        PlaneHit { plane_id: 3, x_mm: 0.0, y_mm: 0.0, z_mm: 225.0, charge_adc: 1000, timestamp_ps: 210 },
    ];

    let track = fitter.fit_hits(&hits).expect("Kalman fit failed");
    assert!(track.chi2_ndf < 1.0);
    assert!((track.direction.z.abs() - 1.0).abs() < 1e-3);
}

#[test]
fn test_poca_intersection() {
    let p_in = Point3::new(0.0, 0.0, 300.0);
    let d_in = Vector3::new(0.0, 0.0, -1.0);

    let p_out = Point3::new(0.0, 0.0, -300.0);
    let d_out = Vector3::new(0.0, 0.0, -1.0);

    let (poca, miss, theta) = calculate_poca_intersection(&p_in, &d_in, &p_out, &d_out);
    assert_eq!(poca, Point3::new(0.0, 0.0, 0.0));
    assert!(miss < 1e-3);
    assert!(theta < 1e-3);
}

#[test]
fn test_live_voxel_accumulator() {
    let mut grid = LiveVoxelAccumulator::new(
        (16, 16, 16),
        Point3::new(-100.0, -100.0, -100.0),
        Point3::new(100.0, 100.0, 100.0),
    );

    let event = CorrelatedEvent {
        event_id: 1,
        ingress_origin: Point3::new(0.0, 0.0, 100.0),
        ingress_dir: Vector3::new(0.0, 0.0, -1.0),
        egress_origin: Point3::new(5.0, 0.0, -100.0),
        egress_dir: Vector3::new(0.05, 0.0, -0.998).normalize(),
        poca_point: Point3::new(2.5, 0.0, 0.0),
        scattering_angle_rad: 0.05,
        miss_distance_mm: 1.0,
        timestamp_utc_ns: 1000,
    };

    assert!(grid.deposit_event(&event));
}

#[test]
fn test_flux_normalizer() {
    let norm = FluxNormalizer::default();
    let scale = norm.compute_scale_factor(1003.25, 15.0);
    assert!(scale < 1.0);
    assert!((scale - (1.0 / 1.015)).abs() < 1e-3);
}

#[test]
fn test_georeferencing_transformation() {
    let attitude = StationAttitude::new(0.0, 0.0, 100.0, 0.0, 0.0, 0.0);
    let p_local = Point3::new(0.0, 0.0, 0.0);
    let p_global = attitude.transform_local_to_global(&p_local);
    assert_eq!(p_global, Point3::new(0.0, 0.0, 100.0));
}

#[test]
fn test_channel_deskew_calibration() {
    let mut calibrator = ChannelDeskewCalibrator::new(4);
    let raw_delays = vec![1200.0, 1122.0, 1044.0, 1200.0];
    let taps = calibrator.compute_optimal_taps(&raw_delays);
    assert_eq!(taps.len(), 4);
    assert!(calibrator.is_calibration_passed());
}
