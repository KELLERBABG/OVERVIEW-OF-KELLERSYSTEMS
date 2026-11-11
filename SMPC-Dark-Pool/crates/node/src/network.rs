use crate::config::{NetworkConfig, NodeConfig};
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use uuid::Uuid;

type SharedConnections = Arc<Mutex<HashMap<Uuid, mpsc::Sender<Vec<u8>>>>>;

pub type MessageCallback = Box<dyn Fn(&[u8], &Uuid) + Send + Sync>;

async fn read_frame<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    max_message_size: usize,
) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > max_message_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "message too large",
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_frame<W: AsyncWriteExt + Unpin>(writer: &mut W, data: &[u8]) -> std::io::Result<()> {
    let len = data.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

pub struct NetworkManager {
    pub node_id: Uuid,
    connections: SharedConnections,
    callbacks: Arc<Mutex<Vec<MessageCallback>>>,
    message_tx: mpsc::Sender<Vec<u8>>,
    message_rx: Option<mpsc::Receiver<Vec<u8>>>,
    incoming_tx: mpsc::Sender<(Vec<u8>, Uuid)>,
    incoming_rx: Option<mpsc::Receiver<(Vec<u8>, Uuid)>>,
    _accept_handle: Option<tokio::task::JoinHandle<()>>,
    listen_addr: String,
    peers: Vec<(Uuid, String)>,
    net_config: NetworkConfig,
}

impl NetworkManager {
    pub fn new(config: &NodeConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(config.net_config.channel_size);
        let (incoming_tx, incoming_rx) = mpsc::channel(config.net_config.channel_size);
        let peers: Vec<(Uuid, String)> = config
            .peers
            .iter()
            .map(|p| (p.node_id, p.address.clone()))
            .collect();

        Self {
            node_id: config.node_id,
            connections: Arc::new(Mutex::new(HashMap::new())),
            callbacks: Arc::new(Mutex::new(Vec::new())),
            message_tx,
            message_rx: Some(message_rx),
            incoming_tx,
            incoming_rx: Some(incoming_rx),
            _accept_handle: None,
            listen_addr: config.listen_addr.clone(),
            peers,
            net_config: config.net_config,
        }
    }

    pub fn message_rx(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        self.message_rx.take()
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        let node_id = self.node_id;
        let net_config = self.net_config;

        let incoming_rx = self
            .incoming_rx
            .take()
            .expect("NetworkManager::start called twice");
        let d_callbacks = self.callbacks.clone();
        let d_message_tx = self.message_tx.clone();
        tokio::spawn(async move {
            Self::dispatch_loop(incoming_rx, d_message_tx, d_callbacks).await;
        });

        for (peer_id, addr) in &self.peers {
            let peer_id = *peer_id;
            let peer_addr = addr.clone();
            let our_id = node_id;
            let inc_tx = self.incoming_tx.clone();
            let conns = self.connections.clone();
            tokio::spawn(async move {
                Self::connection_handler(peer_id, peer_addr, our_id, inc_tx, conns, net_config)
                    .await;
            });
        }

        let listen_addr = self.listen_addr.clone();
        let inc_tx = self.incoming_tx.clone();
        let conns = self.connections.clone();
        let handle = tokio::spawn(async move {
            Self::accept_loop(listen_addr, inc_tx, conns, net_config).await;
        });
        self._accept_handle = Some(handle);

        Ok(())
    }

    async fn dispatch_loop(
        mut incoming_rx: mpsc::Receiver<(Vec<u8>, Uuid)>,
        message_tx: mpsc::Sender<Vec<u8>>,
        callbacks: Arc<Mutex<Vec<MessageCallback>>>,
    ) {
        while let Some((data, sender_id)) = incoming_rx.recv().await {
            {
                let cbs = callbacks.lock().unwrap();
                for cb in cbs.iter() {
                    cb(&data, &sender_id);
                }
            }
            if message_tx.send(data).await.is_err() {
                break;
            }
        }
    }

    async fn accept_loop(
        listen_addr: String,
        incoming_tx: mpsc::Sender<(Vec<u8>, Uuid)>,
        connections: SharedConnections,
        net_config: NetworkConfig,
    ) {
        let listener = match TcpListener::bind(&listen_addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("failed to bind {listen_addr}: {e}");
                return;
            }
        };

        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    let handshake = match tokio::time::timeout(
                        Duration::from_millis(net_config.handshake_timeout_ms),
                        read_frame(&mut stream, net_config.max_message_size),
                    )
                    .await
                    {
                        Ok(Ok(data)) => data,
                        Ok(Err(e)) => {
                            tracing::warn!("handshake read from {addr} failed: {e}");
                            continue;
                        }
                        Err(_) => {
                            tracing::warn!("handshake timeout from {addr}");
                            continue;
                        }
                    };
                    if handshake.len() != 16 {
                        tracing::warn!("invalid handshake from {addr}: wrong length");
                        continue;
                    }
                    let peer_id = match Uuid::from_slice(&handshake) {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::warn!("invalid handshake UUID from {addr}: {e}");
                            continue;
                        }
                    };

                    let (write_tx, write_rx) = mpsc::channel(net_config.channel_size);
                    connections.lock().unwrap().insert(peer_id, write_tx);

                    let (read_half, write_half) = stream.into_split();
                    let inc_tx_clone = incoming_tx.clone();

                    tokio::spawn(async move {
                        Self::read_loop(
                            read_half,
                            peer_id,
                            inc_tx_clone,
                            net_config.max_message_size,
                        )
                        .await;
                    });
                    tokio::spawn(async move {
                        Self::write_loop(write_half, write_rx).await;
                    });
                }
                Err(e) => {
                    tracing::warn!("accept error: {e}");
                }
            }
        }
    }

    async fn connection_handler(
        peer_id: Uuid,
        peer_addr: String,
        our_node_id: Uuid,
        incoming_tx: mpsc::Sender<(Vec<u8>, Uuid)>,
        connections: SharedConnections,
        net_config: NetworkConfig,
    ) {
        let mut retry_ms = net_config.initial_retry_interval_ms;
        loop {
            let (write_tx, write_rx) = mpsc::channel(net_config.channel_size);
            connections.lock().unwrap().insert(peer_id, write_tx);

            match TcpStream::connect(&peer_addr).await {
                Ok(mut stream) => {
                    retry_ms = net_config.initial_retry_interval_ms;
                    let handshake_result = tokio::time::timeout(
                        Duration::from_millis(net_config.handshake_timeout_ms),
                        write_frame(&mut stream, our_node_id.as_bytes()),
                    )
                    .await;
                    match handshake_result {
                        Ok(Ok(())) => {
                            let (read_half, write_half) = stream.into_split();
                            let inc_tx = incoming_tx.clone();
                            tokio::spawn(async move {
                                Self::read_loop(
                                    read_half,
                                    peer_id,
                                    inc_tx,
                                    net_config.max_message_size,
                                )
                                .await;
                            });
                            Self::write_loop(write_half, write_rx).await;
                        }
                        _ => {
                            connections.lock().unwrap().remove(&peer_id);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("connect to {peer_id} at {peer_addr} failed: {e}");
                    connections.lock().unwrap().remove(&peer_id);
                }
            }

            connections.lock().unwrap().remove(&peer_id);
            tokio::time::sleep(Duration::from_millis(retry_ms)).await;
            retry_ms = (retry_ms * 2).min(net_config.max_retry_interval_ms);
            retry_ms += rand::thread_rng().gen_range(0..retry_ms / 2);
        }
    }

    async fn read_loop(
        mut read_half: tokio::net::tcp::OwnedReadHalf,
        peer_id: Uuid,
        incoming_tx: mpsc::Sender<(Vec<u8>, Uuid)>,
        max_message_size: usize,
    ) {
        loop {
            match read_frame(&mut read_half, max_message_size).await {
                Ok(data) => {
                    if incoming_tx.send((data, peer_id)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::debug!("read from {peer_id} failed: {e}");
                    break;
                }
            }
        }
    }

    async fn write_loop(
        mut write_half: tokio::net::tcp::OwnedWriteHalf,
        mut rx: mpsc::Receiver<Vec<u8>>,
    ) {
        let batch_size = 10;
        let batch_timeout = Duration::from_millis(10);
        let mut buffer = Vec::with_capacity(batch_size);
        loop {
            buffer.clear();
            if let Some(msg) = rx.recv().await {
                buffer.push(msg);
                let deadline = tokio::time::sleep(batch_timeout);
                tokio::pin!(deadline);
                loop {
                    tokio::select! {
                        msg = rx.recv() => {
                            match msg {
                                Some(m) => {
                                    buffer.push(m);
                                    if buffer.len() >= batch_size {
                                        break;
                                    }
                                }
                                None => break,
                            }
                        }
                        _ = &mut deadline => break,
                    }
                }
            } else {
                break;
            }
            for data in &buffer {
                if write_frame(&mut write_half, data).await.is_err() {
                    return;
                }
            }
        }
    }

    pub fn send(&self, target: &Uuid, data: &[u8]) -> Result<(), String> {
        let connections = self.connections.lock().unwrap();
        match connections.get(target) {
            Some(tx) => tx.try_send(data.to_vec()).map_err(|e| e.to_string()),
            None => Err(format!("no connection to peer {}", target)),
        }
    }

    pub fn broadcast(&self, data: &[u8]) -> Vec<Result<(), String>> {
        let connections = self.connections.lock().unwrap();
        let keys: Vec<_> = connections.keys().copied().collect();
        drop(connections);
        keys.iter()
            .map(|peer_id| self.send(peer_id, data))
            .collect()
    }

    pub fn register_callback(&mut self, cb: MessageCallback) {
        self.callbacks.lock().unwrap().push(cb);
    }

    pub fn local_send(&self, data: Vec<u8>) {
        let _ = self.message_tx.try_send(data);
    }

    pub fn peer_count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }

    pub fn broadcast_pipelined(&self, data: &[u8]) -> usize {
        let connections = self.connections.lock().unwrap();
        let keys: Vec<_> = connections.keys().copied().collect();
        drop(connections);
        let count = keys.len();
        for peer_id in keys {
            if let Err(e) = self.send(&peer_id, data) {
                tracing::warn!("pipelined send to {peer_id} failed: {e}");
            }
        }
        count
    }

    pub fn pipelined_send(&self, target: &Uuid, data: &[u8]) {
        if let Err(e) = self.send(target, data) {
            tracing::warn!("pipelined send to {target} failed: {e}");
        }
    }

    pub fn batch_send(&self, target: &Uuid, messages: &[Vec<u8>]) -> Result<(), String> {
        let connections = self.connections.lock().unwrap();
        match connections.get(target) {
            Some(tx) => {
                for msg in messages {
                    tx.try_send(msg.clone()).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            None => Err(format!("no connection to peer {}", target)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PeerInfo;
    use std::net::TcpListener as StdTcpListener;

    fn make_config() -> NodeConfig {
        let peer = PeerInfo::new(Uuid::new_v4(), "127.0.0.1:9999".into(), "pk".into());
        NodeConfig::new("127.0.0.1:9998".into(), "sk".into(), vec![peer], 3, 5)
    }

    #[test]
    fn test_network_manager_initializes() {
        let config = make_config();
        let manager = NetworkManager::new(&config);
        assert_eq!(manager.node_id, config.node_id);
    }

    #[test]
    fn test_send_to_unknown_peer_fails() {
        let config = make_config();
        let manager = NetworkManager::new(&config);
        let result = manager.send(&Uuid::new_v4(), b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_peer_count_starts_at_zero() {
        let config = make_config();
        let manager = NetworkManager::new(&config);
        assert_eq!(manager.peer_count(), 0);
    }

    #[test]
    fn test_local_send_and_message_rx() {
        let config = make_config();
        let mut manager = NetworkManager::new(&config);
        let mut rx = manager.message_rx().expect("should have rx");
        manager.local_send(b"hello".to_vec());
        let msg = rx.try_recv().expect("should have message");
        assert_eq!(msg, b"hello");
    }

    #[test]
    fn test_broadcast_pipelined_returns_count() {
        let config = make_config();
        let manager = NetworkManager::new(&config);
        let count = manager.broadcast_pipelined(b"test");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_batch_send_to_unknown_peer_fails() {
        let config = make_config();
        let manager = NetworkManager::new(&config);
        let msgs = vec![b"msg1".to_vec(), b"msg2".to_vec()];
        let result = manager.batch_send(&Uuid::new_v4(), &msgs);
        assert!(result.is_err());
    }

    #[test]
    fn test_pipelined_send_unknown_peer() {
        let config = make_config();
        let manager = NetworkManager::new(&config);
        manager.pipelined_send(&Uuid::new_v4(), b"test");
    }

    #[tokio::test]
    async fn test_integration_connect_and_send() {
        let listener1 = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let port1 = listener1.local_addr().unwrap().port();
        drop(listener1);

        let listener2 = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let port2 = listener2.local_addr().unwrap().port();
        drop(listener2);

        let addr1 = format!("127.0.0.1:{port1}");
        let addr2 = format!("127.0.0.1:{port2}");

        let peer1_id = Uuid::new_v4();
        let peer2_id = Uuid::new_v4();

        let mut config1 = NodeConfig::new(addr1.clone(), "sk1".into(), vec![], 1, 1);
        config1.node_id = peer1_id;
        let mut config2 = NodeConfig::new(addr2.clone(), "sk2".into(), vec![], 1, 1);
        config2.node_id = peer2_id;

        config1
            .peers
            .push(PeerInfo::new(peer2_id, addr2.clone(), "pk2".into()));
        config2
            .peers
            .push(PeerInfo::new(peer1_id, addr1.clone(), "pk1".into()));

        let mut mgr1 = NetworkManager::new(&config1);
        let mut mgr2 = NetworkManager::new(&config2);

        mgr1.start().await.unwrap();
        mgr2.start().await.unwrap();

        tokio::time::sleep(Duration::from_millis(500)).await;

        let msg = b"hello from node1";
        let send_result = mgr1.send(&peer2_id, msg);
        assert!(
            send_result.is_ok(),
            "send should succeed: {:?}",
            send_result
        );

        let mut rx2 = mgr2.message_rx().expect("mgr2 should have rx");
        let received = tokio::time::timeout(Duration::from_secs(5), rx2.recv())
            .await
            .expect("timeout waiting for message")
            .expect("channel should not be closed");
        assert_eq!(received, msg);
    }
}
