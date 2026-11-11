//! # Verification Events Protocol
//!
//! Periodic decentralized liveness validation protocol.
//! See WHITEPAPER.md Section 7.2 for design rationale.

use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::constants::*;
use crate::{NodeId, SybilError, SybilResult};

/// Status of a verification event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStatus {
    Scheduled,
    Open,
    Closed,
    Completed { attestation_count: usize },
    Failed { reason: String },
}

/// A scheduled verification event.
#[derive(Debug, Clone)]
pub struct VerificationEvent {
    pub event_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub opens_at: DateTime<Utc>,
    pub closes_at: DateTime<Utc>,
    pub status: EventStatus,
    pub registered: HashSet<NodeId>,
    pub participants: HashSet<NodeId>,
    pub threshold: u32,
}

/// A liveness attestation signed by a participant.
#[derive(Debug, Clone)]
pub struct LivenessAttestation {
    pub event_id: Uuid,
    pub attester: NodeId,
    pub attested_nodes: Vec<NodeId>,
    pub signature: String,
    pub timestamp: DateTime<Utc>,
}

/// Manages the lifecycle of verification events.
pub struct EventsManager {
    events: Vec<VerificationEvent>,
    attestations: HashMap<Uuid, Vec<LivenessAttestation>>,
    missed_events: HashMap<NodeId, u32>,
    interval_days: i64,
    min_participants: u32,
    max_missed_allowed: u32,
}

impl EventsManager {
    pub fn new(interval_days: i64, min_participants: u32, max_missed_allowed: u32) -> Self {
        Self {
            events: Vec::new(),
            attestations: HashMap::new(),
            missed_events: HashMap::new(),
            interval_days,
            min_participants,
            max_missed_allowed,
        }
    }

    pub fn schedule_event(&mut self) -> VerificationEvent {
        let now = Utc::now();
        let event = VerificationEvent {
            event_id: Uuid::new_v4(),
            created_at: now,
            opens_at: now + Duration::days(1),
            closes_at: now + Duration::days(self.interval_days),
            status: EventStatus::Scheduled,
            registered: HashSet::new(),
            participants: HashSet::new(),
            threshold: self.min_participants,
        };
        self.events.push(event.clone());
        event
    }

    pub fn register_for_event(&mut self, event_id: &Uuid, node_id: &NodeId) -> SybilResult<()> {
        let event = self.events.iter_mut().find(|e| e.event_id == *event_id)
            .ok_or_else(|| SybilError::Api(format!("Event {} not found", event_id)))?;
        if event.status != EventStatus::Scheduled && event.status != EventStatus::Open {
            return Err(SybilError::Api("Event is not open for registration".to_string()));
        }
        event.registered.insert(node_id.clone());
        event.status = EventStatus::Open;
        Ok(())
    }

    pub fn record_attestation(&mut self, event_id: &Uuid, attestation: LivenessAttestation) -> SybilResult<()> {
        let event = self.events.iter_mut().find(|e| e.event_id == *event_id)
            .ok_or_else(|| SybilError::Api(format!("Event {} not found", event_id)))?;
        event.participants.insert(attestation.attester.clone());
        self.attestations.entry(*event_id).or_default().push(attestation.clone());
        Ok(())
    }

    pub fn close_event(&mut self, event_id: &Uuid) -> SybilResult<()> {
        let event = self.events.iter_mut().find(|e| e.event_id == *event_id)
            .ok_or_else(|| SybilError::Api(format!("Event {} not found", event_id)))?;
        let pc = event.participants.len() as u32;
        if pc >= event.threshold {
            event.status = EventStatus::Completed { attestation_count: pc as usize };
            for p in &event.participants { self.missed_events.remove(p); }
        } else {
            event.status = EventStatus::Failed { reason: format!("Only {} participants, needed {}", pc, event.threshold) };
        }
        Ok(())
    }

    pub fn record_missed(&mut self, node_id: &NodeId) -> u32 {
        let missed = self.missed_events.entry(node_id.clone()).or_insert(0);
        *missed += 1;
        *missed
    }

    pub fn should_penalize(&self, node_id: &NodeId) -> bool {
        self.missed_events.get(node_id).copied().unwrap_or(0) >= self.max_missed_allowed
    }

    pub fn all_events(&self) -> &[VerificationEvent] { &self.events }
    pub fn missed_count(&self, node_id: &NodeId) -> u32 {
        self.missed_events.get(node_id).copied().unwrap_or(0)
    }
}

impl Default for EventsManager {
    fn default() -> Self {
        Self::new(VERIFICATION_EVENT_INTERVAL_DAYS, MIN_VERIFICATION_PARTICIPANTS, 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_event() {
        let mut mgr = EventsManager::default();
        let event = mgr.schedule_event();
        assert_eq!(event.status, EventStatus::Scheduled);
        assert_eq!(mgr.all_events().len(), 1);
    }

    #[test]
    fn test_registration() {
        let mut mgr = EventsManager::default();
        let event = mgr.schedule_event();
        let node = NodeId::new();
        mgr.register_for_event(&event.event_id, &node).unwrap();
    }

    #[test]
    fn test_event_completion() {
        let mut mgr = EventsManager::new(30, 3, 3);
        let event = mgr.schedule_event();
        let nodes: Vec<NodeId> = (0..5).map(|_| NodeId::new()).collect();
        for n in &nodes {
            mgr.register_for_event(&event.event_id, n).unwrap();
            mgr.record_attestation(&event.event_id, LivenessAttestation {
                event_id: event.event_id,
                attester: n.clone(),
                attested_nodes: nodes.clone(),
                signature: "sig".into(),
                timestamp: Utc::now(),
            }).unwrap();
        }
        mgr.close_event(&event.event_id).unwrap();
        let closed = mgr.all_events().iter().find(|e| e.event_id == event.event_id).unwrap();
        assert!(matches!(closed.status, EventStatus::Completed { .. }));
    }

    #[test]
    fn test_event_failure_not_enough() {
        let mut mgr = EventsManager::new(30, 5, 3);
        let event = mgr.schedule_event();
        let node = NodeId::new();
        mgr.register_for_event(&event.event_id, &node).unwrap();
        mgr.record_attestation(&event.event_id, LivenessAttestation {
            event_id: event.event_id, attester: node.clone(), attested_nodes: vec![], signature: "sig".into(), timestamp: Utc::now(),
        }).unwrap();
        mgr.close_event(&event.event_id).unwrap();
        let closed = mgr.all_events().iter().find(|e| e.event_id == event.event_id).unwrap();
        assert!(matches!(closed.status, EventStatus::Failed { .. }));
    }

    #[test]
    fn test_missed_event_tracking() {
        let mut mgr = EventsManager::default();
        let node = NodeId::new();
        assert_eq!(mgr.missed_count(&node), 0);
        mgr.record_missed(&node);
        mgr.record_missed(&node);
        mgr.record_missed(&node);
        assert_eq!(mgr.missed_count(&node), 3);
        assert!(mgr.should_penalize(&node));
    }
}