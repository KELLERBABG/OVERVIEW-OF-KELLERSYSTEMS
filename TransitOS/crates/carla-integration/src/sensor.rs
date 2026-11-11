use transit_ros2_integration::messages::ObstacleMsg;
use transit_taint_analysis::sensors::{SensorSource, SensorType};
use transit_taint_analysis::taint::TaintedValue;

pub struct SensorFusion {
    pub radar_sources: Vec<SensorSource>,
    pub lidar_sources: Vec<SensorSource>,
    pub camera_sources: Vec<SensorSource>,
}

impl SensorFusion {
    pub fn new() -> Self {
        Self {
            radar_sources: vec![SensorSource {
                sensor: SensorType::Radar,
                id: "carla_radar_0".into(),
            }],
            lidar_sources: vec![SensorSource {
                sensor: SensorType::Lidar,
                id: "carla_lidar_0".into(),
            }],
            camera_sources: vec![SensorSource {
                sensor: SensorType::Camera,
                id: "carla_camera_0".into(),
            }],
        }
    }

    pub fn fuse_obstacles(
        &self,
        radar_obs: &[ObstacleMsg],
        lidar_obs: &[ObstacleMsg],
        camera_obs: &[ObstacleMsg],
    ) -> Vec<TaintedValue<ObstacleMsg>> {
        let mut fused: Vec<TaintedValue<ObstacleMsg>> = Vec::new();

        for obs in radar_obs {
            fused.push(TaintedValue::from_source(
                obs.clone(),
                self.radar_sources[0].clone(),
            ));
        }
        for obs in lidar_obs {
            fused.push(TaintedValue::from_source(
                obs.clone(),
                self.lidar_sources[0].clone(),
            ));
        }
        for obs in camera_obs {
            fused.push(TaintedValue::tainted(
                obs.clone(),
                SensorType::Camera,
                "carla_camera_0",
            ));
        }

        fused
    }
}

impl Default for SensorFusion {
    fn default() -> Self {
        Self::new()
    }
}
