use crate::query::CollisionQuery;

pub fn query_to_smt2(query: &CollisionQuery) -> String {
    let header = "(set-logic QF_NRA)\n(set-option :produce-models true)\n".to_string();

    let vars = ["t", "ego_x", "ego_y", "obs_x", "obs_y"];

    let decls: String = vars
        .iter()
        .map(|v| format!("(declare-fun {} () Real)", v))
        .collect::<Vec<_>>()
        .join("\n");

    let t_horizon = query.time_horizon;
    let (ex, ey) = query.ego_position;
    let ev = query.ego_velocity;
    let eh = query.ego_heading;

    let body = if let Some(obs) = query.obstacles.first() {
        let (ox, oy) = obs.position;
        let combined_r = 3.0;
        format!(
            "(assert
  (and
    (<= 0 t {})
    (= ego_x (+ {} (* {} (cos {}) t)))
    (= ego_y (+ {} (* {} (sin {}) t)))
    (= obs_x {})
    (= obs_y {})
    (<= (+ (* (- ego_x obs_x) (- ego_x obs_x))
           (* (- ego_y obs_y) (- ego_y obs_y)))
        (* {} {}))))",
            t_horizon, ex, ev, eh, ey, ev, eh, ox, oy, combined_r, combined_r
        )
    } else {
        "(assert true)".to_string()
    };

    format!("{}\n{}\n{}\n(check-sat)\n(get-model)", header, decls, body)
}
