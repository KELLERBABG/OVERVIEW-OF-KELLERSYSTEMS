use transit_state_machine::params::RoadSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultParty {
    Ego,
    Other,
    Unavoidable,
}

#[derive(Debug, Clone)]
pub struct RssRule {
    pub name: &'static str,
    pub description: &'static str,
}

impl RssRule {
    pub fn longitudinal_safe_distance(
        ego_velocity: f64,
        ego_max_decel: f64,
        lead_velocity: f64,
        lead_max_decel: f64,
        reaction_time: f64,
    ) -> f64 {
        let d_ego =
            ego_velocity * reaction_time + (ego_velocity * ego_velocity) / (2.0 * ego_max_decel);
        let d_lead = (lead_velocity * lead_velocity) / (2.0 * lead_max_decel);
        (d_ego - d_lead).max(0.0) + 2.0
    }

    pub fn lateral_safe_distance(ego_velocity: f64, lateral_jerk_limit: f64) -> f64 {
        let lat_time = 1.0;
        (ego_velocity * lat_time + 0.5 * lateral_jerk_limit * lat_time * lat_time).abs()
    }

    pub fn cut_in_violation(
        ego_velocity: f64,
        _ego_max_decel: f64,
        current_gap: f64,
        reaction_time: f64,
        road: &RoadSurface,
    ) -> (bool, FaultParty) {
        let max_dec = road.max_deceleration();
        let safe_gap =
            ego_velocity * reaction_time + (ego_velocity * ego_velocity) / (2.0 * max_dec);
        if current_gap < safe_gap {
            (true, FaultParty::Other)
        } else {
            (false, FaultParty::Ego)
        }
    }

    pub fn rear_end_fault(
        ego_velocity: f64,
        ego_max_decel: f64,
        lead_velocity: f64,
        lead_max_decel: f64,
        gap: f64,
    ) -> FaultParty {
        let min_stop = Self::longitudinal_safe_distance(
            ego_velocity,
            ego_max_decel,
            lead_velocity,
            lead_max_decel,
            0.5,
        );
        if gap < min_stop {
            FaultParty::Ego
        } else {
            FaultParty::Unavoidable
        }
    }
}

pub struct RssViolatorDetector;

impl RssViolatorDetector {
    pub fn check_cut_in(
        ego_velocity: f64,
        lateral_gap: f64,
        longitudinal_gap: f64,
        road: &RoadSurface,
        reaction_time: f64,
    ) -> (bool, FaultParty, Option<&'static str>) {
        let safe_lat = RssRule::lateral_safe_distance(ego_velocity, 2.0);
        if lateral_gap < safe_lat {
            return (
                true,
                FaultParty::Other,
                Some("Vehicle cut in below lateral safe distance"),
            );
        }
        let max_dec = road.max_deceleration();
        let safe_long =
            ego_velocity * reaction_time + (ego_velocity * ego_velocity) / (2.0 * max_dec);
        if longitudinal_gap < safe_long && lateral_gap < safe_lat * 2.0 {
            return (
                true,
                FaultParty::Other,
                Some("Vehicle cut in within stopping distance"),
            );
        }
        (false, FaultParty::Ego, None)
    }

    pub fn should_trigger_mrm(
        fault: FaultParty,
        current_speed: f64,
        obstacle_distance: f64,
        max_decel: f64,
    ) -> bool {
        match fault {
            FaultParty::Other | FaultParty::Unavoidable => {
                let min_stop = (current_speed * current_speed) / (2.0 * max_decel);
                obstacle_distance < min_stop * 1.5
            }
            FaultParty::Ego => false,
        }
    }

    pub fn determine_fault(
        cut_in: bool,
        cut_in_fault: FaultParty,
        rear_end_gap_ok: bool,
    ) -> FaultParty {
        if cut_in {
            cut_in_fault
        } else if !rear_end_gap_ok {
            FaultParty::Ego
        } else {
            FaultParty::Unavoidable
        }
    }
}
