use bytes::Bytes;
use shared::game_message::{GameMessage, PeerType};

#[allow(unused)]
use log::{debug, info, warn, error};

mod pubsub;

use crate::pubsub::PubSub;

#[allow(unused)]
fn test_pub_sub_direct_calls()
{
    let mut pub_sub: PubSub = PubSub::default();


    pub_sub.subscribe(123, "aaa");
    pub_sub.subscribe(456, "aaa");
    pub_sub.subscribe(789, "bbb");

    info!("topic_subs: {:?}\n", pub_sub.topic_subs);


    pub_sub.publish("aaa", &[0x11, 0x22, 0x33]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.publish("aaa", &[0x11, 0x22, 0x33]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    
    pub_sub.publish("aaa", &[0x11, 0x22, 0x33, 0x44]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    
    pub_sub.publish("bbb", &[0x33, 0x22, 0x11]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    

    pub_sub.unsubscribe(456, "aaa");

    info!("topic_subs: {:?}\n", pub_sub.topic_subs);


    pub_sub.publish("aaa", &[0x11, 0x22, 0x33]);
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);
    
    pub_sub.publish("xyz", &[0x12,0x34,0x56,0x78,0x90]);

    info!("topics: {:?}", pub_sub.topic_data);

    return;
}

#[allow(unused)]
fn test_pub_sub_process_packets()
{
    let mut pub_sub: PubSub = PubSub::default();

    // Big Endian, so [0x01, 0x02, 0x03, 0x04] is read as 0x01_02_03_04
    //
    // 0x01_02_03_04 == 16_909_060
    
    // ---------------------------------------------------------------------------------------------------------------
    
    // Since we don't pass the peer_socket_id directly in the function we call, we need to register them first.

    pub_sub.process_received_packet(
        123_u128,
        Bytes::from_static(
            // register(u8), peer_type(u8), peer_id(u32)
            &[GameMessage::REGISTER,/**/ PeerType::CLIENT,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,
            ]
    ));

    pub_sub.process_received_packet(
        456_u128,
        Bytes::from_static(
            // register(u8), peer_type(u8), peer_id(u32)
            &[GameMessage::REGISTER,/**/ PeerType::GAME_SERVER,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,
            ]
    ));

    pub_sub.process_received_packet(
        789_u128,
        Bytes::from_static(
            // register(u8), peer_type(u8), peer_id(u32)
            &[GameMessage::REGISTER,/**/ PeerType::OTHER_SERVER,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,
            ]
    ));

    // all 3 have the same ID, so we can test if the Peer Types works as intended
    // register complete
    // ---------------------------------------------------------------------------------------------------------------

    pub_sub.process_received_packet(
        0_u128, // doesn't matter who subscribes, what matters is who is being subscribed
        Bytes::from_static(
            // subscribe(u8), peer_type(u8), peer_id(u32), topic_size(u16), topic(&str)
            &[GameMessage::SUBSCRIBE,/**/ PeerType::CLIENT,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a']
    )); // CLIENT 0x01020304... = peer_socket_id 123 (being subscribed by 000 but we don't care by who it is subscribed)

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            &[GameMessage::SUBSCRIBE,/**/ PeerType::GAME_SERVER,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a']
    )); // SERVER 0x01020304... = peer_socket_id 456

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            &[GameMessage::SUBSCRIBE,/**/ PeerType::OTHER_SERVER,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,/**/ 0x00, 0x03,/**/ b'b', b'b', b'b']
    )); // OTHER 0x01020304 = peer_socket_id 789

    // subscribes completed
    info!("topic_subs: {:?}\n", pub_sub.topic_subs);
    // ---------------------------------------------------------------------------------------------------------------

    pub_sub.process_received_packet(
        0_u128, // doesn't matter who publishes
        Bytes::from_static(
            // publish(u8), topic_size(u16), topic(&str), data_size(u16), data(&[u8])
            &[GameMessage::PUBLISH,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a',/**/ 0x00, 0x03,/**/ 0x11, 0x22, 0x33]
    ));
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            &[GameMessage::PUBLISH,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a',/**/ 0x00, 0x03,/**/ 0x11, 0x22, 0x33] // same data as previous publish
    ));
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            &[GameMessage::PUBLISH,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a',/**/ 0x00, 0x04,/**/ 0x11, 0x22, 0x33, 0x44] // data is longer
    ));
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            &[GameMessage::PUBLISH,/**/ 0x00, 0x03,/**/ b'b', b'b', b'b',/**/ 0x00, 0x03,/**/ 0x11, 0x22, 0x33] // topic is different
    ));
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    // ---------------------------------------------------------------------------------------------------------------

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            // unsubscribe(u8), peer_type(u8), peer_id(u32), topic_size(u16), topic(&str)
            &[GameMessage::UNSUBSCRIBE,/**/ PeerType::GAME_SERVER,/**/
              0x01, 0x02, 0x03, 0x04,
              0x05, 0x06, 0x03, 0x04,
              0x09, 0x0a, 0x0b, 0x0c,
              0x0d, 0x0e, 0x0f, 0x10,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a']
    ));
    
    info!("topic_subs: {:?}\n", pub_sub.topic_subs);
    
    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            // publish(u8), topic_size(u16), topic(&str), data_size(u16), data(&[u8])
            &[GameMessage::PUBLISH,/**/ 0x00, 0x03,/**/ b'a', b'a', b'a',/**/ 0x00, 0x03,/**/ 0x11, 0x22, 0x33]
    ));
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    pub_sub.process_received_packet(
        0_u128,
        Bytes::from_static(
            // publish(u8), topic_size(u16), topic(&str), data_size(u16), data(&[u8])
            &[GameMessage::PUBLISH,/**/ 0x00, 0x03,/**/ b'x', b'y', b'z',/**/ 0x00, 0x05,/**/ 0x12, 0x34, 0x56, 0x78, 0x90]
    ));
    PubSub::flush_peer_buffers(&mut pub_sub.subs_buffers);

    
    info!("topics: {:?}", pub_sub.topic_data);

    return;
}


fn main()
{
    // allow info!() logging without needing to set any environment variables
    env_logger::Builder::new().filter_level(
        log::LevelFilter::Info
        // log::LevelFilter::Debug
    ).parse_default_env().init();
    
    println!("Hello, world!\n");

    // test_pub_sub_direct_calls();
    test_pub_sub_process_packets();

    println!("\nGoodbye, world!");
}
