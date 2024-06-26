use libp2p::{gossipsub, Multiaddr, PeerId};
use tokio::sync::{mpsc, oneshot};

use super::{stream, Command};

#[derive(Clone)]
pub struct NetworkClient {
    pub(crate) sender: mpsc::Sender<Command>,
}

impl NetworkClient {
    pub async fn listen_on(&self, addr: Multiaddr) -> eyre::Result<()> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::ListenOn { addr, sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn dial(&self, peer_addr: Multiaddr) -> eyre::Result<Option<()>> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::Dial { peer_addr, sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn subscribe(
        &self,
        topic: gossipsub::IdentTopic,
    ) -> eyre::Result<gossipsub::IdentTopic> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::Subscribe { topic, sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn unsubscribe(
        &self,
        topic: gossipsub::IdentTopic,
    ) -> eyre::Result<gossipsub::IdentTopic> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::Unsubscribe { topic, sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn publish(
        &self,
        topic: gossipsub::TopicHash,
        data: Vec<u8>,
    ) -> eyre::Result<gossipsub::MessageId> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::Publish {
                topic,
                data,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn open_stream(&self, peer_id: PeerId) -> eyre::Result<stream::Stream> {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::OpenStream { peer_id, sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn peer_info(&self) -> super::PeersInfo {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::PeersInfo { sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn mesh_peer_info(&self, topic: gossipsub::TopicHash) -> super::MeshPeersInfo {
        let (sender, receiver) = oneshot::channel();

        self.sender
            .send(Command::MeshPeersInfo { topic, sender })
            .await
            .expect("Command receiver not to be dropped.");

        receiver.await.expect("Sender not to be dropped.")
    }
}
