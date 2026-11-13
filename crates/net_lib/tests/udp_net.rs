use net_lib::udp_net::{UdpReceiver, UdpSender, UdpNet};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
enum Packet {
    Option1(u32),
    Option2(u64),
}

#[test]
fn udp_net_simple() {
    let (mut s1, mut s2) = get_two_sockets::<Packet>(net_lib::udp_net::LOOPBACK_BUF_SIZE);

    let sent_packet = Packet::Option1(255);
    s1.send(&sent_packet).unwrap();

    let packet = s2.recv().unwrap();
    assert_eq!(packet, sent_packet);

    let sent_packet = Packet::Option2(200);
    s2.send(&sent_packet).unwrap();

    let packet = s1.recv().unwrap();
    assert_eq!(packet, sent_packet);
}

#[test]
fn udp_net_split() {
    let (s1, s2) = get_two_sockets::<Packet>(net_lib::udp_net::LOOPBACK_BUF_SIZE);
    let (mut sender1, mut receiver1) = s1.split().unwrap();
    let (mut sender2, mut receiver2) = s2.split().unwrap();

    let sent_packet = Packet::Option1(593);
    sender1.send(&sent_packet).unwrap();

    let packet = receiver2.recv().unwrap();
    assert_eq!(packet, sent_packet);
    sender2.send(&packet).unwrap();

    let packet = receiver1.recv().unwrap();
    assert_eq!(packet, sent_packet);

    // Address checks
    let addr1 = sender1.local_addr().unwrap();
    assert_eq!(receiver1.local_addr().unwrap(), addr1);

    let peer1 = sender1.peer_addr().unwrap();
    assert_eq!(receiver1.peer_addr().unwrap(), peer1);
    assert!(addr1 != peer1);

    let addr2 = sender2.local_addr().unwrap();
    assert_eq!(receiver2.local_addr().unwrap(), addr2);

    let peer2 = sender2.peer_addr().unwrap();
    assert_eq!(receiver2.peer_addr().unwrap(), peer2);
    assert!(addr2 != peer2);

    assert!(addr1 != addr2);
    assert!(peer1 != peer2);
    assert_eq!(addr1, peer2);
    assert_eq!(addr2, peer1);
}

/// Get two connected sockets.
fn get_two_sockets<P>(buf_size: usize) -> (UdpNet<P>, UdpNet<P>)
where
    P: Serialize + DeserializeOwned,
{
    let socket1 = UdpNet::bind("localhost:0", buf_size).unwrap();
    let addr1 = socket1.local_addr().unwrap();
    let socket2 = UdpNet::bind("localhost:0", buf_size).unwrap();

    socket2.connect(addr1).unwrap();
    assert_eq!(socket2.peer_addr().unwrap(), addr1);

    let addr2 = socket2.local_addr().unwrap();
    socket1.connect(addr2).unwrap();
    assert_eq!(socket1.peer_addr().unwrap(), addr2);

    (socket1, socket2)
}
