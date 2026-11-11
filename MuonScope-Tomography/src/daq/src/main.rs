use std::time::Instant;
use muonscope_daq::barycenter::TriangularBarycenter;
use muonscope_daq::coordinate_frame::StationAttitude;
use muonscope_daq::flux_normalizer::FluxNormalizer;
use muonscope_daq::packet_decoder::PACKET_SYNC_WORD;
use muonscope_daq::poca_core::{calculate_poca_intersection, LiveVoxelAccumulator};
use muonscope_daq::ring_buffer::SpscRingBuffer;
use muonscope_daq::storage::EventFileWriter;
use muonscope_daq::track_kalman::KalmanTrackFitter;
use muonscope_daq::{CorrelatedEvent, PlaneHit, Point3};

fn create_synthetic_raw_packet(seq_id: u32, x_offset_mm: f64) -> Vec<u8> {
    use byteorder::{BigEndian, WriteBytesExt};
    use crc32fast::Hasher;

    let mut buf = Vec::new();
    buf.write_u32::<BigEndian>(PACKET_SYNC_WORD).unwrap();
    buf.write_u32::<BigEndian>(seq_id).unwrap();
    buf.write_u64::<BigEndian>(1700000000000000000 + (seq_id as u64) * 1000).unwrap();
    buf.write_u8(1).unwrap();
    buf.write_u8(0).unwrap();
    buf.write_u16::<BigEndian>(0x000F).unwrap();
    buf.write_u16::<BigEndian>(4).unwrap();

    let planes = [(0, 375.0), (1, 365.0), (2, 235.0), (3, 225.0)];
    for (plane_id, _z) in planes {
        let strip = ((x_offset_mm + 250.0) / 20.0).clamp(0.0, 24.0) as u8;
        buf.write_u8(plane_id).unwrap();
        buf.write_u8(strip).unwrap();
        buf.write_u16::<BigEndian>(500).unwrap();
        buf.write_u16::<BigEndian>(1200).unwrap();
    }

    let mut hasher = Hasher::new();
    hasher.update(&buf);
    let crc = hasher.finalize();
    buf.write_u32::<BigEndian>(crc).unwrap();

    buf
}

fn main() {
    println!("============================================================");
    println!("  MUONSCOPE EDGE DAQ & REAL-TIME TRACKING ENGINE v1.0");
    println!("============================================================");

    let start_time = Instant::now();

    let mut ring_buffer = SpscRingBuffer::<Vec<u8>>::new(1024);
    let barycenter = TriangularBarycenter::new(20.0, 25);
    let mut kalman_top = KalmanTrackFitter::new();
    let mut kalman_bottom = KalmanTrackFitter::new();
    let _attitude = StationAttitude::new(48.8566, 2.3522, 120.0, 0.0, 0.0, 0.0);
    let normalizer = FluxNormalizer::default();

    let mut voxel_grid = LiveVoxelAccumulator::new(
        (32, 32, 32),
        Point3::new(-250.0, -250.0, -200.0),
        Point3::new(250.0, 250.0, 200.0),
    );

    let mut writer = EventFileWriter::create("live_events.jsonl").expect("Failed to create event log");

    let total_packets = 5000;
    println!("  Streaming {} raw binary FPGA packets into ring buffer...", total_packets);

    let mut pushed = 0;
    let mut processed_tracks = 0;
    let mut correlated_events = 0;

    for i in 0..total_packets {
        let x_pos = ((i % 20) as f64 - 10.0) * 15.0; // [-150, 150]
        let tx_scatter = (((i % 11) as f64) - 5.0) * 0.005 + 0.02; // [0.015, 0.045] rad
        let pkt = create_synthetic_raw_packet(i as u32, x_pos);
        if ring_buffer.try_push(pkt) {
            pushed += 1;
        }

        if let Some(raw_bytes) = ring_buffer.try_pop() {
            if let Ok(decoded) = muonscope_daq::packet_decoder::decode_fpga_packet(&raw_bytes) {
                let mut top_hits = Vec::new();

                for (plane_id, tdc, adc) in decoded.channel_hits {
                    let strip_idx = ((x_pos + 250.0) / 20.0).clamp(0.0, 24.0) as usize;
                    let pos_x = barycenter.calculate_hit_pos(strip_idx, adc as f64, None, None);

                    let z_mm = match plane_id {
                        0 => 375.0,
                        1 => 365.0,
                        2 => 235.0,
                        _ => 225.0,
                    };

                    top_hits.push(PlaneHit {
                        plane_id,
                        x_mm: pos_x,
                        y_mm: 0.0,
                        z_mm,
                        charge_adc: adc,
                        timestamp_ps: tdc as u64,
                    });
                }

                let top_track = kalman_top.fit_hits(&top_hits);
                
                if let Some(t_in) = top_track {
                    let origin_x = t_in.origin_mm.x;
                    let x_b1 = origin_x - 225.0 * tx_scatter;
                    let x_b2 = origin_x - 365.0 * tx_scatter;

                    let dummy_bottom_hits = vec![
                        PlaneHit { plane_id: 4, x_mm: x_b1, y_mm: 0.0, z_mm: -225.0, charge_adc: 1100, timestamp_ps: 1000 },
                        PlaneHit { plane_id: 6, x_mm: x_b2, y_mm: 0.0, z_mm: -365.0, charge_adc: 1150, timestamp_ps: 1200 },
                    ];
                    let bottom_track = kalman_bottom.fit_hits(&dummy_bottom_hits);

                    if let Some(t_out) = bottom_track {
                        processed_tracks += 2;

                        let (poca, miss_dist, theta) = calculate_poca_intersection(
                            &t_in.origin_mm,
                            &t_in.direction,
                            &t_out.origin_mm,
                            &t_out.direction,
                        );

                        let event = CorrelatedEvent {
                            event_id: decoded.sequence_id as u64,
                            ingress_origin: t_in.origin_mm,
                            ingress_dir: t_in.direction,
                            egress_origin: t_out.origin_mm,
                            egress_dir: t_out.direction,
                            poca_point: poca,
                            scattering_angle_rad: theta,
                            miss_distance_mm: miss_dist,
                            timestamp_utc_ns: decoded.timestamp_utc_ns,
                        };

                        if voxel_grid.deposit_event(&event) {
                            let _ = writer.write_event(&event);
                            correlated_events += 1;
                        }
                    }
                }
            }
        }
    }

    writer.flush().unwrap();
    let duration = start_time.elapsed();
    let throughput = (pushed as f64) / duration.as_secs_f64();
    let scale_factor = normalizer.compute_scale_factor(1005.0, 22.0);

    println!("------------------------------------------------------------");
    println!("  Total Ingested Packets:     {}", pushed);
    println!("  Reconstructed Tracks:       {}", processed_tracks);
    println!("  Correlated PoCA Events:     {}", correlated_events);
    println!("  Processing Duration:        {:.3} s", duration.as_secs_f64());
    println!("  Throughput Rate:            {:.0} packets/sec", throughput);
    println!("  Barometric Flux Scale:      {:.4}", scale_factor);
    println!("============================================================");
}
