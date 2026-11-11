use std::collections::HashMap;
use crate::TrackSegment;

pub struct SamTMeshCorrelator {
    max_time_window_ns: u64,
    ingress_tracks: HashMap<u8, Vec<TrackSegment>>,
}

impl SamTMeshCorrelator {
    pub fn new(max_time_window_ns: u64) -> Self {
        Self {
            max_time_window_ns,
            ingress_tracks: HashMap::new(),
        }
    }

    pub fn insert_track(&mut self, station_id: u8, track: TrackSegment) {
        self.ingress_tracks.entry(station_id).or_default().push(track);
    }

    pub fn correlate_pair(
        &mut self,
        station_a: u8,
        station_b: u8,
    ) -> Vec<(TrackSegment, TrackSegment)> {
        let mut matched = Vec::new();

        if let (Some(tracks_a), Some(tracks_b)) = (
            self.ingress_tracks.get(&station_a),
            self.ingress_tracks.get(&station_b),
        ) {
            for ta in tracks_a {
                for tb in tracks_b {
                    let dt = if ta.timestamp_utc_ns > tb.timestamp_utc_ns {
                        ta.timestamp_utc_ns - tb.timestamp_utc_ns
                    } else {
                        tb.timestamp_utc_ns - ta.timestamp_utc_ns
                    };

                    if dt <= self.max_time_window_ns {
                        matched.push((ta.clone(), tb.clone()));
                    }
                }
            }
        }

        matched
    }
}
