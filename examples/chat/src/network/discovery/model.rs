use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::time::Duration;

use libp2p::{rendezvous, PeerId, StreamProtocol};
use multiaddr::Multiaddr;
use tokio::time;

// The rendezvous protocol name is not public in libp2p, so we have to define it here.
// source: https://github.com/libp2p/rust-libp2p/blob/a8888a7978f08ec9b8762207bf166193bf312b94/protocols/rendezvous/src/lib.rs#L50C12-L50C92
const RENDEZVOUS_PROTOCOL_NAME: libp2p::StreamProtocol =
    libp2p::StreamProtocol::new("/rendezvous/1.0.0");

#[derive(Debug)]
pub(crate) struct DiscoveryState {
    peers: BTreeMap<PeerId, PeerInfo>,
    relay_index: BTreeSet<PeerId>,
    rendezvous_index: BTreeSet<PeerId>,
    pending_addr_changes: bool,
}

impl DiscoveryState {
    pub(crate) fn new() -> Self {
        DiscoveryState {
            peers: Default::default(),
            relay_index: Default::default(),
            rendezvous_index: Default::default(),
            pending_addr_changes: false,
        }
    }

    pub(crate) fn add_peer_addr(&mut self, peer_id: PeerId, addr: &Multiaddr) {
        self.peers
            .entry(peer_id)
            .or_insert_with(|| PeerInfo {
                addrs: Default::default(),
                relay: None,
                rendezvous: None,
            })
            .addrs
            .insert(addr.clone());
    }

    pub(crate) fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
        self.relay_index.remove(peer_id);
        self.rendezvous_index.remove(peer_id);
    }

    pub(crate) fn is_peer_of_interest(&self, peer_id: &PeerId) -> bool {
        self.relay_index.contains(peer_id) || self.rendezvous_index.contains(peer_id)
    }

    pub(crate) fn update_peer_protocols(
        &mut self,
        peer_id: &PeerId,
        protocols: Vec<StreamProtocol>,
    ) -> eyre::Result<()> {
        protocols.iter().for_each(|protocol| {
            if protocol == &libp2p::relay::HOP_PROTOCOL_NAME {
                self.relay_index.insert(*peer_id);
                self.peers.entry(*peer_id).or_default().relay = Some(PeerRelayInfo {
                    reservation_status: Default::default(),
                });
            }
            if protocol == &RENDEZVOUS_PROTOCOL_NAME {
                self.rendezvous_index.insert(*peer_id);
                self.peers.entry(*peer_id).or_default().rendezvous = Some(PeerRendezvousInfo {
                    cookie: None,
                    last_discovery_at: None,
                });
            }
        });
        Ok(())
    }

    pub(crate) fn update_rendezvous_cookie(
        &mut self,
        rendezvous_peer: &PeerId,
        cookie: rendezvous::Cookie,
    ) -> eyre::Result<()> {
        self.peers
            .entry(*rendezvous_peer)
            .and_modify(|info| info.update_rendezvous_cookie(cookie));
        Ok(())
    }

    pub(crate) fn update_relay_reservation_status(
        &mut self,
        relay_peer: &PeerId,
        status: RelayReservationStatus,
    ) -> eyre::Result<()> {
        self.peers
            .entry(*relay_peer)
            .and_modify(|info| info.update_relay_reservation_status(status));
        Ok(())
    }

    pub(crate) fn get_peer_info(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.peers.get(peer_id)
    }

    pub(crate) fn get_rendezvous_peer_ids(&self) -> impl Iterator<Item = PeerId> + '_ {
        self.rendezvous_index.iter().cloned()
    }

    pub(crate) fn is_peer_relay(&self, peer_id: &PeerId) -> bool {
        self.relay_index.contains(peer_id)
    }

    pub(crate) fn is_peer_rendezvous(&self, peer_id: &PeerId) -> bool {
        self.rendezvous_index.contains(peer_id)
    }

    pub(crate) fn pending_addr_changes(&self) -> bool {
        self.pending_addr_changes
    }

    pub(crate) fn set_pending_addr_changes(&mut self) {
        self.pending_addr_changes = true;
    }

    pub(crate) fn clear_pending_addr_changes(&mut self) {
        self.pending_addr_changes = true;
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PeerInfo {
    addrs: HashSet<Multiaddr>,
    relay: Option<PeerRelayInfo>,
    rendezvous: Option<PeerRendezvousInfo>,
}

impl PeerInfo {
    pub(crate) fn addrs(&self) -> impl Iterator<Item = &Multiaddr> {
        self.addrs.iter()
    }

    pub(crate) fn get_preferred_addr(&self) -> Option<Multiaddr> {
        let udp_addrs: Vec<&Multiaddr> = self
            .addrs
            .iter()
            .filter(|addr| {
                addr.iter()
                    .any(|p| matches!(p, multiaddr::Protocol::Udp(_)))
            })
            .collect();

        match udp_addrs.len() {
            0 => self.addrs.iter().next().cloned(),
            _ => Some(udp_addrs[0].clone()),
        }
    }

    pub(crate) fn rendezvous_cookie(&self) -> Option<&rendezvous::Cookie> {
        self.rendezvous.as_ref().and_then(|info| info.cookie())
    }

    pub(crate) fn update_rendezvous_cookie(&mut self, cookie: rendezvous::Cookie) {
        if let Some(ref mut rendezvous_info) = self.rendezvous {
            rendezvous_info.update_cookie(cookie);
        }
    }

    pub(crate) fn is_rendezvous_discovery_time(&self) -> bool {
        self.rendezvous
            .as_ref()
            .map_or(false, |info| info.should_discover())
    }

    pub(crate) fn relay_reservation_status(&self) -> Option<RelayReservationStatus> {
        self.relay
            .as_ref()
            .and_then(|info| Some(info.reservation_status()))
    }

    pub(crate) fn update_relay_reservation_status(&mut self, status: RelayReservationStatus) {
        if let Some(ref mut relay_info) = self.relay {
            relay_info.update_reservation_status(status);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PeerRelayInfo {
    reservation_status: RelayReservationStatus,
}

impl PeerRelayInfo {
    pub(crate) fn reservation_status(&self) -> RelayReservationStatus {
        self.reservation_status
    }

    pub(crate) fn update_reservation_status(&mut self, status: RelayReservationStatus) {
        self.reservation_status = status;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum RelayReservationStatus {
    #[default]
    Discovered,
    Requested,
    Accepted,
}

#[derive(Clone, Debug)]
pub(crate) struct PeerRendezvousInfo {
    cookie: Option<rendezvous::Cookie>,
    last_discovery_at: Option<time::Instant>,
}

impl PeerRendezvousInfo {
    pub(crate) fn cookie(&self) -> Option<&rendezvous::Cookie> {
        self.cookie.as_ref()
    }

    pub(crate) fn update_cookie(&mut self, cookie: rendezvous::Cookie) {
        self.cookie = Some(cookie);
        self.last_discovery_at = Some(time::Instant::now());
    }

    pub(crate) fn should_discover(&self) -> bool {
        match self.last_discovery_at {
            Some(instant) => instant.elapsed() > Duration::from_secs(60),
            None => true,
        }
    }
}
