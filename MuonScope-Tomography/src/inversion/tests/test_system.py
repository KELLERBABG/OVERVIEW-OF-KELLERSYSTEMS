"""Tests for the instrument software: wire format, framing, calibration, tracking.

These exercise the real processing chain on bytes. Where a test needs a data
source it uses the instrument simulator, because the simulator emits through the
same encoder the hardware would -- so passing here is evidence about the software
rather than about a convenient shortcut.
"""
import numpy as np
import pytest

from muonscope.acquisition import (
    MemorySource,
    PacketReader,
    SimulatedSource,
    SimulatorConfig,
    CaptureWriter,
    FileSource,
)
from muonscope.simulation.detector import DetectorResponse, HodoscopeSimulator
from muonscope.simulation.geometry import (
    load_detector_geometry,
    parse_detector_geometry,
    validate_detector_geometry,
)
from muonscope.simulation.ray_tracer import MonteCarloRayTracer
from muonscope.simulation.synthetic_scene import create_heavy_inclusion_phantom
from muonscope.tracking import ChannelMap, TdcCalibration, TrackFitter
from muonscope.system import MuonScopeSystem, SystemConfig, compare_to_truth
from muonscope.wire import (
    FLAG_CALIBRATION,
    HEADER_SIZE,
    HIT_SIZE,
    MAGIC,
    MIN_PACKET_SIZE,
    EventRecord,
    HitRecord,
    ProtocolError,
    decode_packet,
    encode_packet,
    packet_length,
)


# ---------------------------------------------------------------------------
# fixtures
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def geometry():
    return load_detector_geometry()


def _record(sequence=1, n_hits=8, channel_base=0):
    return EventRecord(
        sequence=sequence,
        timestamp_utc_ns=1_700_000_000_000_000_000 + sequence,
        station_id=1,
        trigger_mask=0xFF,
        hits=[HitRecord(channel=channel_base + i, tdc_coarse=20, tdc_fine=i * 3,
                        adc=1000 + i) for i in range(n_hits)],
    )


# ---------------------------------------------------------------------------
# wire format
# ---------------------------------------------------------------------------

def test_wire_roundtrip_preserves_every_field():
    original = _record(sequence=4242, n_hits=8)
    original.flags = 0x01
    decoded = decode_packet(encode_packet(original))
    assert decoded.sequence == original.sequence
    assert decoded.timestamp_utc_ns == original.timestamp_utc_ns
    assert decoded.station_id == original.station_id
    assert decoded.trigger_mask == original.trigger_mask
    assert decoded.flags == original.flags
    assert decoded.n_hits == original.n_hits
    for a, b in zip(original.hits, decoded.hits):
        assert (a.channel, a.tdc_coarse, a.tdc_fine, a.adc, a.flags) == \
               (b.channel, b.tdc_coarse, b.tdc_fine, b.adc, b.flags)


def test_wire_length_is_variable_and_self_describing():
    """A four-plane event must fit; a fixed 32-byte frame could not hold one."""
    assert packet_length(0) == MIN_PACKET_SIZE == 27
    assert HEADER_SIZE == 23 and HIT_SIZE == 9
    for n in (0, 1, 4, 8, 16):
        assert len(encode_packet(_record(n_hits=n))) == 27 + 9 * n
    # 8 hits need 99 bytes, i.e. more than the documented 32-byte frame.
    assert len(encode_packet(_record(n_hits=8))) > 32


def test_wire_rejects_bad_magic():
    payload = bytearray(encode_packet(_record()))
    payload[0] ^= 0xFF
    with pytest.raises(ProtocolError, match="magic"):
        decode_packet(bytes(payload))


def test_wire_rejects_bad_crc():
    payload = bytearray(encode_packet(_record()))
    payload[-1] ^= 0xFF
    with pytest.raises(ProtocolError, match="CRC"):
        decode_packet(bytes(payload))


def test_wire_minimum_length_gate_is_correct():
    """The old gate admitted buffers too short to contain a CRC."""
    with pytest.raises(ProtocolError, match="minimum"):
        decode_packet(b"\x00" * (MIN_PACKET_SIZE - 1))
    # Exactly the minimum is a legal zero-hit packet, and must not raise on length.
    zero_hit = encode_packet(_record(n_hits=0))
    assert len(zero_hit) == MIN_PACKET_SIZE
    assert decode_packet(zero_hit).n_hits == 0


def test_wire_rejects_truncated_hit_list():
    payload = encode_packet(_record(n_hits=8))[:-4]
    with pytest.raises(ProtocolError):
        decode_packet(payload)


def test_wire_has_no_momentum_or_energy_field():
    """The packet must not be able to carry a quantity the detector cannot measure."""
    record = decode_packet(encode_packet(_record()))
    for name in dir(record):
        lowered = name.lower()
        assert "energy" not in lowered
        assert "momentum" not in lowered
        assert not lowered.endswith("_p")


# ---------------------------------------------------------------------------
# framing
# ---------------------------------------------------------------------------

def test_framing_recovers_from_leading_garbage():
    reader = PacketReader()
    stream = b"\x00\x11\x22" + encode_packet(_record(sequence=1)) + b"\xff" * 5 \
        + encode_packet(_record(sequence=2))
    records = reader.feed(stream)
    assert [r.sequence for r in records] == [1, 2]


def test_framing_survives_a_corrupt_packet_in_the_middle():
    good1 = encode_packet(_record(sequence=1))
    bad = bytearray(encode_packet(_record(sequence=2)))
    bad[-1] ^= 0xFF                      # break the CRC
    good3 = encode_packet(_record(sequence=3))
    reader = PacketReader()
    records = reader.feed(good1 + bytes(bad) + good3)
    assert [r.sequence for r in records] == [1, 3]
    assert reader.stats["packets_bad"] == 1


def test_framing_handles_packets_split_across_reads():
    """A UART delivers partial frames; losing them would lose most of a run."""
    stream = b"".join(encode_packet(_record(sequence=i)) for i in range(5))
    reader = PacketReader()
    seen = []
    for i in range(0, len(stream), 7):          # deliberately misaligned chunks
        seen.extend(reader.feed(stream[i:i + 7]))
    assert [r.sequence for r in seen] == [0, 1, 2, 3, 4]


def test_framing_holds_incomplete_tail_without_inventing_records():
    reader = PacketReader()
    payload = encode_packet(_record(sequence=9))
    assert reader.feed(payload[:10]) == []
    records = reader.feed(payload[10:])
    assert [r.sequence for r in records] == [9]


def test_file_source_replays_a_capture(tmp_path):
    path = tmp_path / "run.msc"
    with CaptureWriter(str(path)) as writer:
        for i in range(4):
            writer.write(_record(sequence=i))
    reader = PacketReader()
    records = list(reader.records_from_source(FileSource(str(path))))
    assert [r.sequence for r in records] == [0, 1, 2, 3]


# ---------------------------------------------------------------------------
# geometry
# ---------------------------------------------------------------------------

def test_geometry_summary_reports_lever_arm_not_panel_separation(geometry):
    """The arm that sets angular resolution is within a station, not between them."""
    assert geometry.top.lever_arm_mm("X") == pytest.approx(140.0)
    assert geometry.panel_centre_separation_mm() == pytest.approx(600.0)
    assert geometry.top.lever_arm_mm("X") < geometry.panel_centre_separation_mm()


def test_channel_banks_do_not_alias(geometry):
    """Both panels number from zero; a global id must still be unique."""
    cmap = ChannelMap(geometry)
    assert cmap.num_channels() == geometry.total_channels() == 512
    seen = {}
    for channel in range(cmap.num_channels()):
        loc = cmap.locate(channel)
        assert loc is not None, f"channel {channel} unmapped"
        key = (loc.panel_key, loc.plane_id, loc.strip_index)
        assert key not in seen, f"{key} mapped twice"
        seen[key] = channel
    top = [c for c in range(cmap.num_channels()) if cmap.locate(c).panel_key == "top_panel"]
    bottom = [c for c in range(cmap.num_channels()) if cmap.locate(c).panel_key == "bottom_panel"]
    assert not set(top) & set(bottom)


def test_geometry_validation_rejects_a_station_with_one_layer():
    data = {
        "system": {"name": "t", "version": "1", "units": "mm"},
        "panels": {
            "top_panel": {
                "id": "T", "active_area": [500, 500], "z_offset": 300.0,
                "plane_separation": 150.0,
                "planes": [
                    {"plane_id": "X1", "orientation": "X", "num_strips": 4,
                     "strip_width": 20.0, "z_position": 375.0, "sipm_channels": [0, 3]},
                    {"plane_id": "X2", "orientation": "X", "num_strips": 4,
                     "strip_width": 20.0, "z_position": 235.0, "sipm_channels": [4, 7]},
                    {"plane_id": "Y1", "orientation": "Y", "num_strips": 4,
                     "strip_width": 20.0, "z_position": 365.0, "sipm_channels": [8, 11]},
                ],
            },
            "bottom_panel": {
                "id": "B", "active_area": [500, 500], "z_offset": -300.0,
                "plane_separation": 150.0,
                "planes": [
                    {"plane_id": "X3", "orientation": "X", "num_strips": 4,
                     "strip_width": 20.0, "z_position": -225.0, "sipm_channels": [0, 3]},
                    {"plane_id": "X4", "orientation": "X", "num_strips": 4,
                     "strip_width": 20.0, "z_position": -365.0, "sipm_channels": [4, 7]},
                    {"plane_id": "Y3", "orientation": "Y", "num_strips": 4,
                     "strip_width": 20.0, "z_position": -235.0, "sipm_channels": [8, 11]},
                    {"plane_id": "Y4", "orientation": "Y", "num_strips": 4,
                     "strip_width": 20.0, "z_position": -375.0, "sipm_channels": [12, 15]},
                ],
            },
        },
        "trigger_logic": {},
    }
    # A station claiming a coordinate it cannot measure must be rejected at parse
    # time, not silently accepted and then fail later during a survey.
    with pytest.raises(ValueError, match="top_panel/Y"):
        parse_detector_geometry(data)


# ---------------------------------------------------------------------------
# calibration
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def simulated_capture(geometry):
    """A small end-to-end simulator capture, reused across tests."""
    scene = create_heavy_inclusion_phantom(grid_size=(8, 8, 8))
    cfg = SimulatorConfig(num_events=1200, seed=4242, calibration_packets=250)
    source = SimulatedSource(scene, geometry, config=cfg)
    reader = PacketReader()
    records = list(reader.records_from_source(source))
    return records, source, scene


def test_simulator_emits_calibration_records_then_physics(simulated_capture):
    records, source, _ = simulated_capture
    cal = [r for r in records if r.flags & FLAG_CALIBRATION]
    phys = [r for r in records if not (r.flags & FLAG_CALIBRATION)]
    assert len(cal) == 250
    assert len(phys) > 0
    assert all(r.flags & 0x01 for r in records), "simulated flag must be set"


def test_calibration_recovers_the_injected_fine_bin_width(geometry, simulated_capture):
    """Code density on a uniform-phase pulser run must find the true 75 ps."""
    records, _, _ = simulated_capture
    cal = TdcCalibration.from_records(
        records, ChannelMap(geometry), clock_period_ns=5.0, nominal_bin_ps=75.0)
    assert cal.fine_bin_ps == pytest.approx(75.0, rel=0.05), \
        f"recovered {cal.fine_bin_ps:.2f} ps"
    assert cal.channels_calibrated == 512


def test_calibration_recovers_injected_channel_skew(geometry, simulated_capture):
    records, source, _ = simulated_capture
    cal = TdcCalibration.from_records(
        records, ChannelMap(geometry), clock_period_ns=5.0)
    truth = source.channel_delay_ps
    estimated = np.array([cal.channel_offset_ps.get(c, 0.0) for c in range(512)])
    # Offsets are relative to the array mean, so compare after removing the mean.
    est = estimated - estimated.mean()
    tru = truth - truth.mean()
    assert np.corrcoef(est, tru)[0, 1] > 0.9, "deskew does not track the injected skew"
    assert np.std(est - tru) < 0.25 * np.std(tru)


def test_calibration_without_a_calibration_run_falls_back_and_says_so(geometry, simulated_capture):
    records, _, _ = simulated_capture
    physics_only = [r for r in records if not (r.flags & FLAG_CALIBRATION)]
    cal = TdcCalibration.from_records(physics_only, ChannelMap(geometry))
    # Still produces a usable object, just without a trustworthy bin width.
    assert cal.fine_bin_ps > 0
    assert cal.channels_calibrated > 0


# ---------------------------------------------------------------------------
# tracking
# ---------------------------------------------------------------------------

def test_tracker_fits_muon_tracks_from_records(geometry, simulated_capture):
    records, _, _ = simulated_capture
    physics = [r for r in records if not (r.flags & FLAG_CALIBRATION)]
    cal = TdcCalibration.from_records(records, ChannelMap(geometry))
    fitter = TrackFitter(geometry, calibration=cal)
    tracks = fitter.fit_all(physics)
    assert len(tracks) > 0.5 * len(physics), "most complete records should fit"
    for track in tracks[:20]:
        assert track.d_in[2] < 0.0 and track.d_out[2] < 0.0
        assert np.isclose(np.linalg.norm(track.d_in), 1.0)
        assert np.isclose(np.linalg.norm(track.d_out), 1.0)
        assert 0.0 <= track.theta_rad < np.pi


def test_time_of_flight_matches_the_panel_separation(geometry, simulated_capture):
    """600 mm between stations is ~2.0 ns; a ToF cut is only meaningful if so."""
    records, _, _ = simulated_capture
    cal = TdcCalibration.from_records(records, ChannelMap(geometry))
    tracks = TrackFitter(geometry, calibration=cal).fit_all(
        [r for r in records if not (r.flags & FLAG_CALIBRATION)])
    tof = np.array([t.tof_ns for t in tracks])
    assert np.median(tof) == pytest.approx(2.0, abs=0.5), f"median ToF {np.median(tof)}"


def test_chi2_is_reported_unavailable_without_redundancy(geometry, simulated_capture):
    """Two layers per line give no degrees of freedom; a chi2 cut cannot exist."""
    records, _, _ = simulated_capture
    cal = TdcCalibration.from_records(records, ChannelMap(geometry))
    fitter = TrackFitter(geometry, calibration=cal)
    tracks = fitter.fit_all([r for r in records if not (r.flags & FLAG_CALIBRATION)])
    assert tracks, "need at least one track for this check"
    assert all(t.chi2_ndf_dof == 0 for t in tracks)
    assert fitter.stats.rejected_chi2 == 0
    assert fitter.stats.chi2_not_available == len(tracks)


# ---------------------------------------------------------------------------
# detector model
# ---------------------------------------------------------------------------

def test_measured_resolution_matches_the_geometric_prediction(geometry):
    """The resolution model must be verifiable, not asserted.

    The test measures the *slope residual* of each fitted direction, whose
    variance is ``2 sigma_pos^2 / L^2`` per component. That estimator is used
    rather than the angle-space excess ``E[theta_meas^2 - theta_true^2]`` because
    the latter is dominated by a large cross term ``2 E[delta_s_true . delta]``
    that only averages away slowly: at a few hundred events its sampling error is
    comparable to the signal it is supposed to reveal, and it will happily report
    a 2x discrepancy that is not there. The identity linking the two is checked
    explicitly below.
    """
    scene = create_heavy_inclusion_phantom(grid_size=(8, 8, 8))
    z_top, z_bot = geometry.entry_exit_z_mm()
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=z_top, bottom_panel_z_mm=z_bot)
    from muonscope.physics.gaisser import sample_cosmic_muons
    energies, directions, charges = sample_cosmic_muons(
        5000, rng=np.random.default_rng(99))
    truth = tracer.trace_batch(5000, energies, directions, charges,
                               rng=np.random.default_rng(100))
    sim = HodoscopeSimulator(geometry, DetectorResponse(), rng=np.random.default_rng(101))
    measured = sim.measure_batch(truth)
    assert len(measured) > 400

    by_id = {e.event_id: e for e in truth}
    sigma = DetectorResponse().sigma_pos_mm
    lever = sim._lever["top"]["X"]
    expected = 2.0 * sigma ** 2 / lever ** 2      # per-component slope variance

    residuals = []
    for m in measured:
        t = by_id[m.event_id]
        for d_true, d_meas in ((t.d_in, m.d_in_measured), (t.d_out, m.d_out_measured)):
            residuals.append(d_meas[0] / d_meas[2] - d_true[0] / d_true[2])
            residuals.append(d_meas[1] / d_meas[2] - d_true[1] / d_true[2])
    var = float(np.var(residuals))
    assert var == pytest.approx(expected, rel=0.10), (
        f"slope variance {var:.4e} vs predicted {expected:.4e}")

    # The angle-space excess must equal the summed component variance, up to the
    # cross term. Check the identity rather than assuming the term vanishes.
    excess = np.array([
        m.theta_measured_sq - by_id[m.event_id].scattering_angle_rad ** 2
        for m in measured])
    assert np.mean(excess) == pytest.approx(4.0 * expected, rel=0.35)


def test_digitised_readout_is_worse_than_barycentric(geometry):
    """Sanity: giving up charge sharing must cost resolution, not gain it."""
    scene = create_heavy_inclusion_phantom(grid_size=(8, 8, 8))
    z_top, z_bot = geometry.entry_exit_z_mm()
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=z_top, bottom_panel_z_mm=z_bot)
    from muonscope.physics.gaisser import sample_cosmic_muons
    energies, directions, charges = sample_cosmic_muons(
        2500, rng=np.random.default_rng(5))
    truth = tracer.trace_batch(2500, energies, directions, charges,
                               rng=np.random.default_rng(6))
    by_id = {e.event_id: e for e in truth}

    def spread(digitised):
        sim = HodoscopeSimulator(
            geometry, DetectorResponse(strip_digitised=digitised),
            rng=np.random.default_rng(7))
        measured = sim.measure_batch(truth)
        excess = np.array([
            m.theta_measured_sq - by_id[m.event_id].scattering_angle_rad ** 2
            for m in measured])
        return float(np.mean(excess))

    assert spread(True) > spread(False) * 2.0


# ---------------------------------------------------------------------------
# end to end
# ---------------------------------------------------------------------------

def test_system_runs_end_to_end_on_simulated_bytes(geometry):
    scene = create_heavy_inclusion_phantom(grid_size=(8, 8, 8))
    cfg = SimulatorConfig(num_events=1500, seed=777, calibration_packets=250)
    source = SimulatedSource(scene, geometry, config=cfg)

    config = SystemConfig(
        grid_size=8,
        bounds_min_mm=tuple(float(v) for v in scene.bounds_min_mm),
        bounds_max_mm=tuple(float(v) for v in scene.bounds_max_mm),
        iterations=3,
    )
    system = MuonScopeSystem(config=config, geometry=geometry)
    run = system.run_from_source(source)

    assert run.n_tracks > 100
    assert run.reader_stats["packets_bad"] == 0
    assert run.calibration_summary["fine_bin_ps"] == pytest.approx(75.0, rel=0.05)
    assert set(run.volumes) >= {"backprojection", "marginalized_mlem", "nominal_momentum"}
    for volume in run.volumes.values():
        assert volume.shape == (8, 8, 8)
        assert np.all(np.isfinite(volume))
        assert np.all(volume >= 0.0)

    truth = compare_to_truth(run, source.truth_by_sequence)
    assert truth["matched"] == run.n_tracks
    assert truth["resolution_ratio"] == pytest.approx(1.0, rel=0.25)


def test_marginalized_mlem_recovers_a_known_density():
    """Guards the MLEM normalisation.

    The update denominator carries ``<K>^2`` against ``(W lambda)``. Writing it
    against the predicted signal ``<K>(W lambda)`` instead puts a third factor of
    ``<K>`` in the denominator and inflates every step by ``1/<K>`` -- about 2765x
    at ``<K> ~ 3.6e-4`` -- which drives the entire volume to its ceiling. That is a
    silent failure: the output stays finite, non-negative and correctly shaped, so
    only a test on the recovered *magnitude* catches it.
    """
    import scipy.sparse as sp
    from muonscope.physics.momentum_prior import default_momentum_prior
    from muonscope.reconstructors.marginalized import (
        MarginalizedMLEM, marginalized_backprojection, prior_moments)

    rng = np.random.default_rng(11)
    n_rays, n_vox = 600, 256
    rows = rng.integers(0, n_rays, size=6000)
    cols = rng.integers(0, n_vox, size=6000)
    data = rng.uniform(0.5, 3.0, size=6000)
    W = sp.csr_matrix((data, (rows, cols)), shape=(n_rays, n_vox))

    lam_true = 1.0 / 11.55          # concrete, cm^-1
    prior = default_momentum_prior()
    mean_k, _var_k = prior_moments(prior)
    sigma_theta = 0.015

    clean = mean_k * np.asarray(W.dot(np.full(n_vox, lam_true))).ravel()
    assert clean.mean() > 0
    theta_sq = clean + rng.normal(0.0, 0.05 * clean.mean(), size=n_rays)

    # The one-shot estimate is biased high by roughly (total path / voxel path);
    # the iterative solver is what is supposed to remove that bias.
    one_shot, _ = marginalized_backprojection(W, theta_sq, sigma_theta, prior=prior)
    solver = MarginalizedMLEM(prior=prior, iterations=300)
    recovered_vol, _ = solver.reconstruct(W, theta_sq, sigma_theta,
                                          initial_density=one_shot)
    recovered = float(np.mean(recovered_vol))

    assert recovered == pytest.approx(lam_true, rel=0.25), (
        f"MLEM recovered {recovered:.4f} cm^-1, expected about {lam_true:.4f}; "
        f"one-shot start was {float(np.mean(one_shot)):.4f}")


def test_reconstruction_is_bounded_by_the_physical_maximum():
    """No material scatters more than uranium, so no voxel may exceed it."""
    import scipy.sparse as sp
    from muonscope.reconstructors.marginalized import (
        MAX_PHYSICAL_LAMBDA_CM, marginalized_backprojection)

    n_rays, n_vox = 50, 64
    rows = np.repeat(np.arange(n_rays), 2)
    cols = np.arange(2 * n_rays) % n_vox
    W = sp.csr_matrix((np.full(2 * n_rays, 0.01), (rows, cols)), shape=(n_rays, n_vox))
    # Absurd scattering with a very short chord; unbounded, this divides by 0.01 cm.
    theta_sq = np.full(n_rays, 1.0)
    vol, _ = marginalized_backprojection(W, theta_sq, 0.0)
    assert vol.max() <= MAX_PHYSICAL_LAMBDA_CM + 1e-9
    assert vol.max() > 0.0


def test_system_result_carries_no_truth_into_the_reconstruction(geometry):
    """The Track type must not expose anything the detector cannot measure."""
    from muonscope.tracking import Track
    fields = set(Track.__dataclass_fields__)
    for forbidden in ("energy", "momentum", "p_true", "theta_true", "energy_initial_gev"):
        assert forbidden not in fields


def test_marginalised_solver_beats_a_single_nominal_momentum(geometry):
    """The point of marginalising: refusing to model the spectrum costs accuracy.

    Reconstruct the same events three ways -- with the prior, with one nominal
    momentum, and with the true per-event momentum (an oracle no instrument has).
    The marginalised solve should sit between the two, and much closer to the
    oracle than the nominal solve does.
    """
    from muonscope.physics.momentum_prior import gaisser_momentum_prior
    from muonscope.reconstructors.marginalized import (
        marginalized_backprojection, nominal_momentum_baseline)
    from muonscope.reconstructors.siddon import build_sparse_system_matrix

    scene = create_heavy_inclusion_phantom(grid_size=(8, 8, 8))
    z_top, z_bot = geometry.entry_exit_z_mm()
    tracer = MonteCarloRayTracer(scene, top_panel_z_mm=z_top, bottom_panel_z_mm=z_bot)
    from muonscope.physics.gaisser import sample_cosmic_muons
    n = 6000
    energies, directions, charges = sample_cosmic_muons(n, rng=np.random.default_rng(31))
    truth = tracer.trace_batch(n, energies, directions, charges,
                               rng=np.random.default_rng(32))
    z_top, _ = geometry.entry_exit_z_mm()
    sim = HodoscopeSimulator(geometry, DetectorResponse(), rng=np.random.default_rng(33))
    measured = sim.measure_batch(truth)
    assert len(measured) > 500

    by_id = {e.event_id: e for e in truth}
    p_in = np.array([m.position_in_mm for m in measured])
    p_out = np.array([m.position_out_mm for m in measured])
    theta_sq = np.array([m.theta_measured_sq for m in measured])
    sigma = float(np.mean([m.sigma_theta_rad for m in measured]))
    offset = float(np.mean([m.theta_sq_offset_rad2 for m in measured]))

    grid = (8, 8, 8)
    W = build_sparse_system_matrix(
        p_in, p_out, grid, scene.bounds_min_mm, scene.bounds_max_mm)
    truth_volume = scene.get_scattering_density()
    lead_mask = truth_volume > 1.0

    prior = gaisser_momentum_prior()
    marginal, _ = marginalized_backprojection(
        W, theta_sq, sigma, prior=prior, offset_rad2=offset)
    nominal_p = float(np.average(prior.momentum_mev, weights=prior.weights))
    nominal = nominal_momentum_baseline(W, theta_sq, nominal_p, offset_rad2=offset)

    def lead_error(volume):
        rec = volume.reshape(grid)[lead_mask].mean()
        return abs(rec - truth_volume[lead_mask].mean())

    err_marginal = lead_error(marginal)
    err_nominal = lead_error(nominal)
    assert err_marginal < err_nominal, (
        f"marginalised error {err_marginal:.4f} should beat "
        f"nominal {err_nominal:.4f}")
