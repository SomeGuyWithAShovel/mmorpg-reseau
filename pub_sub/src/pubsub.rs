
#[allow(unused)]
use log::{debug, info, warn, error};

use std::collections::HashMap;

use bytes::{
    BufMut, 
    BytesMut,
    Bytes,
    Buf,
};

// -------------------------------------------------------------------------------------------------------------------

pub fn u8_slice_to_hex_string(bytes: &[u8]) -> String
{
    return bytes.iter().map(|b| { format!("{:02x}", *b) }).collect();
}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

/**
    ### PubSubMsgType
    First Byte of packets sent or received by the PubSub
 */
#[allow(unused)]
#[derive(Clone, Copy)]
pub enum PubSubMsgType
{
    None,
    Subscribe,   // Peer -> PubSub
    Unsubscribe, // Peer -> PubSub
    Publish,     // Peer -> PubSub
    Broadcast,   // PubSub -> Peer
    ClientInput, // Client -> PubSub
    Register,    // Peer -> PubSub
}

impl PubSubMsgType
{
    const NUMBER_OF_TYPES: u8 = 6_u8;
    
    #[allow(unused)]
    pub const fn to_u8(&self) -> u8
    {
        let value: u8;
        match self
        {
            Self::Subscribe   => { value = 0x01_u8; }
            Self::Unsubscribe => { value = 0x02_u8; }
            Self::Publish     => { value = 0x03_u8; }
            Self::Broadcast   => { value = 0x04_u8; }
            Self::ClientInput => { value = 0x05_u8; }
            Self::Register    => { value = 0x06_u8; }
            Self::None        => { value = 0x00_u8; }
        }
        return value;
    }

    const FROM_U8_ARRAY: [Self; (Self::NUMBER_OF_TYPES+1) as usize] = [
        Self::None, 
        Self::Subscribe, 
        Self::Unsubscribe,
        Self::Publish, 
        Self::Broadcast, 
        Self::ClientInput,
        Self::Register,
    ];
    
    #[allow(unused)]
    pub const fn from_u8(value: u8) -> Self
    {
        return Self::FROM_U8_ARRAY[ if value <= Self::NUMBER_OF_TYPES { value as usize } else { 0_usize } ]; // /!\ value <= NUMBER_OF_TYPES
    }
}

// -------------------------------------------------------------------------------------------------------------------

/**
    ### PubSubPeerType
    When Subscribing or Unsubscribing, we pass this enum (as an u8) followed by an u32 (PeerId), to tell the PubSub who needs to be subscribed/unsubscribed to a topic.
    This allows for a peer to subscribe another peer (for instance, the spatial server subscribes a client to all other entities in its area of interest), 
    while this other peer (the spatial server, in our example) **does not** need to know the local identifier used by the PubSub's way of communication 
    (GameStream(u128) if using GameSockets) to identify the peer to subscribe (the client, in our example)
 */
#[allow(unused)]
#[derive(Clone, Copy)]
pub enum PubSubPeerType
{
    Client,
    GameServer,
    OtherServer
}

impl PubSubPeerType
{
    const NUMBER_OF_TYPES: u8 = 3_u8;
    
    #[allow(unused)]
    pub const fn to_u8(&self) -> u8
    {
        let index: u8;
        match self
        {
            Self::Client      => { index = 0x00_u8; }
            Self::GameServer  => { index = 0x01_u8; }
            Self::OtherServer => { index = 0x02_u8; }
        }
        return index;
    }
    
    const FROM_USIZE_ARRAY: [Self; Self::NUMBER_OF_TYPES as usize] = [
        Self::Client, 
        Self::GameServer, 
        Self::OtherServer,
    ];
    
    #[allow(unused)]
    pub const fn from_u8(index: u8) -> Option<Self>
    {
        return if index < Self::NUMBER_OF_TYPES { Some(Self::FROM_USIZE_ARRAY[index as usize]) } else { None };
    }

}

// -------------------------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------------------------

type PeerId = u32; // See PubSubPeerType comments

type PeerSocketId = u128; // TODO : replace with GameSockets Connection or Stream or any other Identifier (a tuple of both?). Must be clonable

#[derive(Default)]
pub struct PubSub
{
    pub subs_peer_sockets: [HashMap<PeerId, PeerSocketId>; PubSubPeerType::NUMBER_OF_TYPES as usize],

    pub topic_data: HashMap<String, Vec<u8>>,
    pub topic_subs: HashMap<String, Vec<PeerSocketId>>,

    pub subs_buffers: HashMap<PeerSocketId, BytesMut>
}

// -------------------------------------------------------------------------------------------------------------------

impl PubSub
{
    /**
        Gets the PeerSocketId (used by GameSockets to actually communicate with a peer) from a PubSubPeerType (u8) and a PeerId (u32)
     */
    #[allow(unused)]
    pub fn get_peer_socket_id(&self, peer_type: PubSubPeerType, peer_id: PeerId) -> Option<PeerSocketId>
    {
        return self.subs_peer_sockets[peer_type.to_u8() as usize].get(&peer_id).cloned();
    }

    /**
        Sets a PeerSocketId (used by GameSockets to actually communicate with a peer) that corresponds to a PubSubPeerType (u8) and a PeerId (u32)
    */
    #[allow(unused)]
    pub fn set_peer_socket_id(&mut self, peer_type: PubSubPeerType, peer_id: PeerId, peer_socket_id:  PeerSocketId)
    {
        let insert_result = self.subs_peer_sockets[peer_type.to_u8() as usize].insert(peer_id, peer_socket_id);
        if insert_result.is_some()
        {
            warn!(
                "set_peer_socket_id() : (type {}; peer_id {}) was already present in subs_peer_sockets, with peer_socket_id = {} (it is now set to {})", 
                peer_type.to_u8(), peer_id, insert_result.unwrap(), peer_socket_id
            );
        }
        else
        {
            info!("set_peer_socket_id() : added peer_socket_id {} as type {} and id {}", peer_socket_id, peer_type.to_u8(), peer_id);
        }
        return;
    }

    /**
        Subscribes a PeerSocketId to a Topic
     */
    pub fn subscribe(&mut self, peer_socket_id: PeerSocketId, topic: &str)
    {
        match self.topic_subs.get_mut(topic)
        {
            None => 
            {
                self.topic_subs.insert(topic.to_owned(), vec![peer_socket_id]);
                info!("subscribe() : {} is now subscribed to topic \"{}\"", peer_socket_id, topic);
            }

            Some(subs_vec) => 
            {
                if subs_vec.contains(&peer_socket_id) == false
                {
                    // insert in a sorted way, so we can test contains() in O(log(n)) instead of O(n) ?
                    // (but then in unsubscribe, we can't use swap_remove() in O(1), we would need to remove() in O(n) there) ?

                    subs_vec.push(peer_socket_id);
                    info!("subscribe() : {} is now subscribed to topic \"{}\"", peer_socket_id, topic);
                }
                else
                {
                    warn!("subscribe() : {} is already subscribed to topic \"{}\"", peer_socket_id, topic);
                }
            }
        }
        return;
    }

    /**
        Unsubscribes a PeerSocketId to a Topic
    */
    pub fn unsubscribe(&mut self, peer_socket_id: PeerSocketId, topic: &str)
    {
        match self.topic_subs.get_mut(topic)
        {
            Some(subs_vec) => 
            {
                let Some(found_id) = subs_vec.iter().position(|val| {*val == peer_socket_id} )
                else
                {
                    warn!("unsubscribe() : {} wasn't subscribed to topic \"{}\"", peer_socket_id, topic);
                    return;
                };
                
                // subs_vec[found_id] == peer_socket_id
                subs_vec.swap_remove(found_id);

                info!("unsubscribe() : {} is now unsubscribed to topic \"{}\"", peer_socket_id, topic);
            }

            None => 
            {
                warn!("unsubscribe() : {} wasn't subscribed to topic \"{}\"", peer_socket_id, topic);
            }
        }
        return;
    }

    /**
        Sets the data associated with a Topic, and calls broadcast() on that topic if the data is different from the previously stored data
    */
    pub fn publish(&mut self, topic: &str, data: &[u8])
    {
        // HashMap<K,V>::insert(k,v) : "if the map did have this key present, the value is updated, and the old value is returned."

        let insert_result = self.topic_data.insert(topic.to_owned(), data.to_owned());

        if let Some(old_data) = insert_result // if there were already data, 
        {
            if old_data == data // AND it was the same data, 
            {
                info!("publish() : topic \"{}\" : data published was already present", topic);
                return; // we do nothing
            }
        }
        // else :
        // either there wasn't any data before this call, or there were but it was different data

        info!("publish() : topic \"{}\" : data published : 0x{}", topic, u8_slice_to_hex_string(data));
        self.broadcast(topic);
        return;
    }

    /**
        For all subscribed peers : adds a "subpacket" containing a topic and its new data, inside their buffer (by calling add_subpacket_for_peer).
        
        **Don't forget to flush the buffers to actually send them through the network, otherwise you would just fill an internal buffer !**
     */
    pub fn broadcast(&mut self, topic: &str)
    {
        let Some(subs) = self.topic_subs.get(topic)
        else
        {
            debug!("broadcast() : no one is subscribed to topic \"{}\" : not broadcasting anything", topic);
            return;
        };
        debug!("broadcast() : subscribed to topic \"{}\" : {:?}", topic, subs);

        let data_ref = self.topic_data.get(topic);
        let data_length: usize = if let Some(data) = data_ref { data.len() } else { 0_usize };

        /*
            packet = [ u16,  [u8;...],     u16, [u8;...] ]
                       ^      ^            ^     ^
             topic_length  topic  data_length  data
        */
        let topic_size: usize = size_of::<u16>() + topic.len();
        let data_size: usize = size_of::<u16>() + data_length;

        let mut packet = BytesMut::with_capacity(topic_size + data_size);

        if topic.len() >= u16::MAX as usize { panic!("PubSub::broadcast() : topic.len() >= {}", u16::MAX as usize); }
        packet.put_u16(topic.len() as u16);
        packet.put_slice(topic.as_bytes());

        if data_length >= u16::MAX as usize { panic!("PubSub::broadcast() : data_length >= {}", u16::MAX as usize); }
        packet.put_u16(data_length as u16);
        if data_length > 0
        {
            packet.put_slice(data_ref.unwrap());
        }

        let packet = packet.freeze();

        debug!("broadcast() : packet=0x{}", u8_slice_to_hex_string(&packet));

        for peer_socket_id in subs
        {
            // We only need a mutable reference to the subs buffer, not to the whole PubSub struct.
            // Moreover, if we took the whole PubStruct, we could not compile the program, since we could, in theory,
            // modify the topic_data inside this function, 
            // while we still hold and use (in the next iteration of the loop) non mutable references to the topic_data
            
            Self::add_subpacket_for_peer(&mut self.subs_buffers, *peer_socket_id, &packet);
        }
        return;
    }

    /**
        For the provided peer only, adds inside their buffer a "subpacket".
        If their buffer was empty, adds the necessary header so that the buffer is actually a whole packet containing the right header, followed by the "subpackets"
        
        buffer = [header, subpacket_1, subpacket_2, ...]
     */
    fn add_subpacket_for_peer(subs_buffers: &mut HashMap<PeerSocketId, BytesMut>, peer_socket_id:  PeerSocketId, bytes: &Bytes)
    {
        let get_result = subs_buffers.get_mut(&peer_socket_id);
        if let Some(packet) = get_result
        {
            if packet.len() == 0 // there is a buffer already, but it is empty
            {
                packet.put_u8(PubSubMsgType::Broadcast.to_u8());
            }
            packet.put_slice(bytes);
        }
        else // there isn't any buffer
        {
            let header_size: usize = size_of::<u8>();

            let mut new_packet = BytesMut::with_capacity(header_size + bytes.len());

            new_packet.put_u8(PubSubMsgType::Broadcast.to_u8());

            new_packet.put_slice(bytes);
            
            subs_buffers.insert(peer_socket_id, new_packet);
        }
        debug!("add_subpacket_for_peer() : peer {}: buffer=0x{}", peer_socket_id, u8_slice_to_hex_string(subs_buffers.get(&peer_socket_id).unwrap()));
        return;
    }

    /**
        For all peers, we send them their buffers (it should already be formatted correctly) through the network
     */
    pub fn flush_peer_buffers(subs_buffers: &mut HashMap<PeerSocketId, BytesMut>)
    {
        for (peer_socket_id, packet) in subs_buffers.iter()
        {
            if packet.len() > 0_usize
            {
                println!(
                    "flush_peer_buffers() : sending to {} : {}", 
                    peer_socket_id, u8_slice_to_hex_string(&packet)
                )
                // TODO : send 'packet' to 'peer_socket_id'
            }
        }
        subs_buffers.clear();
        return;
    }

    /**
        "Unserializes" received packets and calls related functions with the wanted parameters
     */
    pub fn process_received_packet(&mut self, peer_socket_id: PeerSocketId, mut bytes: Bytes)
    {
        let raw_msg_type: u8 = bytes.get_u8();
        let msg_type: PubSubMsgType = PubSubMsgType::from_u8(raw_msg_type);
        match msg_type
        {
            PubSubMsgType::None =>
            {
                error!("process_received_packet() : packet type is invalid ({})", raw_msg_type);
            }

            PubSubMsgType::Register => 
            {
                let peer_type_raw = bytes.get_u8();
                let Some(peer_type) = PubSubPeerType::from_u8(peer_type_raw)
                else
                {
                    error!("process_received_packet() : peer type is invalid ({})", peer_type_raw);
                    return;
                };

                let peer_id = bytes.get_u32();

                self.set_peer_socket_id(peer_type, peer_id, peer_socket_id);

            }

            PubSubMsgType::Subscribe | PubSubMsgType::Unsubscribe =>
            {
                let peer_to_sub_type_raw = bytes.get_u8();
                let Some(peer_to_sub_type) = PubSubPeerType::from_u8(peer_to_sub_type_raw)
                else
                {
                    error!("process_received_packet() : peer type is invalid ({})", peer_to_sub_type_raw);
                    return;
                };

                let peer_to_sub_id = bytes.get_u32();

                let Some(peer_to_sub) = self.get_peer_socket_id(peer_to_sub_type, peer_to_sub_id)
                else
                {
                    error!("process_received_packet() : peer to {} couldn't be found (type={}, id={})", 
                        match msg_type { 
                            PubSubMsgType::Subscribe => {"subscribe"} 
                            PubSubMsgType::Unsubscribe => {"unsubscribe"} 
                            _ => { panic!("process_received_packet() : unreachable code reached") } 
                        },
                        peer_to_sub_type_raw, peer_to_sub_id
                    );
                    return;    
                };

                let topic_string_size: usize = bytes.get_u16() as usize;
                let topic_bytes = bytes.split_to(topic_string_size);
                let Ok(topic) = str::from_utf8(&topic_bytes)
                else
                {
                    error!("process_received_packet() : Subscribe/Unsubscribe: Topic received isn't a valid utf-8 &str");
                    return;
                };
                
                match msg_type
                {
                    PubSubMsgType::Subscribe => 
                    {
                        self.subscribe(peer_to_sub, topic);
                    }

                    PubSubMsgType::Unsubscribe =>
                    {
                        self.unsubscribe(peer_to_sub, topic);
                    }
                    _ => { panic!("process_received_packet() : unreachable code reached");}
                }
            }

            PubSubMsgType::Publish =>
            {
                // TODO : check for authority (client can't set some entity's position, only a server can publish its status (CPU usage,...), ...)

                let topic_string_size: usize = bytes.get_u16() as usize; debug!("TOPIC_SIZE: {}", topic_string_size);
                let topic_bytes = bytes.split_to(topic_string_size);
                let Ok(topic) = str::from_utf8(&topic_bytes)
                else
                {
                    error!("process_received_packet() : Subscribe/Unsubscribe: Topic received isn't a valid utf-8 &str");
                    return;
                };

                debug!("process_received_packet() : TOPIC: \"{}\"", topic);

                let data_size: usize = bytes.get_u16() as usize;
                let data_bytes = bytes.split_to(data_size);

                self.publish(topic, &data_bytes);
            }

            PubSubMsgType::Broadcast =>
            {
                error!("process_received_packet() : received a Broadcast message. Broadcast messages should only be sent BY the PubSub, not TO the PubSub")
            }

            PubSubMsgType::ClientInput =>
            {
                warn!("process_received_packet() : received a ClientInput");
            }

        }
    }
    
}
