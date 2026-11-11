# MuonScope Sample Scans & Reference Datasets

This directory contains standardized sample datasets for testing and validating reconstruction algorithms:

1. **`astm_calibration_sample.jsonl`**: 5,000 processed coincidence tracks through the ISO/ASTM E1441 reference cube.
2. **`concrete_rebar_sample.jsonl`**: 10,000 tracks through a reinforced concrete slab with 20 mm rebar grid.
3. **`void_honeycomb_sample.jsonl`**: 8,000 tracks through a bridge pier phantom with internal honeycomb voiding.

### Schema:
Each line is a JSON object with:
- `event_id`: Unique 64-bit event identifier
- `timestamp_utc_ns`: Nanosecond precision UTC timestamp
- `ingress_origin`: `[x, y, z]` in mm
- `ingress_dir`: Normalized `[dx, dy, dz]` unit vector
- `egress_origin`: `[x, y, z]` in mm
- `egress_dir`: Normalized `[dx, dy, dz]` unit vector
- `poca_point`: `[x, y, z]` in mm
- `scattering_angle_rad`: 3D deflection angle in radians
- `miss_distance_mm`: Euclidean distance of closest approach
