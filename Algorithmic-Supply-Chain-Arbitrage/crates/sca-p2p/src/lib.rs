//! Ad-hoc P2P freight capacity orchestration layer.

pub mod network;
pub mod peer;
pub mod protocol;
pub mod transport;

pub use network::P2pNetwork;
pub use peer::{Peer, PeerInfo, PeerId};
pub use protocol::{CapacityOffer, CapacityRequest, Negotiation, NegotiationResult};
pub use transport::{Transport, TransportError, MockTransport};
