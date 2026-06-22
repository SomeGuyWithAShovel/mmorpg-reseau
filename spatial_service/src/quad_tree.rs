
use std::collections::HashMap;

#[allow(unused)]
use log::{debug, info, warn, error};

use bevy_math::Vec2;

use crate::{
    ShardId,
    EntityId,
};

type Coord = f32;

#[derive(Clone, Debug, PartialEq)]
pub struct QTRect
{                 //   .---corner+size
    corner: Vec2, //   |       |
    size: Vec2,   //   |       |
}                 // corner ---'

#[derive(Clone, Copy)]
pub enum QTDir
{
    NE, // +x +y
    NW, // -x +y
    SW, // -x -y
    SE  // +x -y
}

impl QTDir
{
    fn as_usize(&self) -> usize
    {
        match self
        {
            Self::NE => { return 0_usize; }
            Self::NW => { return 1_usize; }
            Self::SW => { return 2_usize; }
            Self::SE => { return 3_usize; }
        }
    }

    fn from_usize(i: usize) -> Option<Self>
    {
        match i
        {
            0_usize => { return Some(Self::NE); }
            1_usize => { return Some(Self::NW); }
            2_usize => { return Some(Self::SW); }
            3_usize => { return Some(Self::SE); }
            _ => { return None; }
        }
    }

    const NUMBER_OF_DIRS: usize = 4_usize;
}

impl QTRect
{
    pub fn center(&self) -> Vec2
    {
        return self.corner + (self.size / (2_f64 as Coord));
    }

    pub fn get_quarter_from_dir(&self, dir: QTDir) -> QTRect
    {
        let new_size = self.size / (2_f64 as Coord);
        let offset: Vec2;
        match dir // all:  :+:
        {
            QTDir::NE => //   +'
            {
                offset = Vec2 {x: new_size.x, y: new_size.y};
                return QTRect{ corner: self.corner + offset, size: new_size };
            },
            QTDir::NW => //  '+
            {
                offset = Vec2 {x: (0_f64 as Coord), y: new_size.y};
                return QTRect{ corner: self.corner + offset, size: new_size };
            },
            QTDir::SW => //  .+
            {
                offset = Vec2 {x: (0_f64 as Coord), y: (0_f64 as Coord)};
                return QTRect{ corner: self.corner + offset, size: new_size };
            },
            QTDir::SE => //   +.
            {
                offset = Vec2 {x: new_size.x, y: (0_f64 as Coord)};
                return QTRect{ corner: self.corner + offset, size: new_size };
            },
        }
    }

    pub fn get_dir_from_pos(&self, pos: Vec2) -> QTDir
    {
        let center = self.center();
        if pos.y >= center.y
        {
            if pos.x >= center.x { return QTDir::NE; }
            else                 { return QTDir::NW; }
        }
        else
        {
            if pos.x <  center.x { return QTDir::SW; }
            else                 { return QTDir::SE; }
        }
    }

    pub fn contains(&self, pos: Vec2) -> bool
    {
        return (self.corner.x <= pos.x                      ) && 
               (pos.x         <= self.corner.x + self.size.x) && 
               (self.corner.y <= pos.y                      ) && 
               (pos.y         <= self.corner.y + self.size.y);
    }

}

struct QTParentNode
{
    pub children: Box<[QTNode; QTDir::NUMBER_OF_DIRS]>,
}

#[derive(Default)]
struct QTLeafNode
{
    pub entities: Vec<EntityId>,
    pub shard_id: ShardId,
}

impl QTLeafNode
{
    // called before inserting an entity (that's why the +1 is here)
    fn should_split(&self) -> bool
    {
        return self.entities.len() + 1_usize > 50_usize; // TODO : replace 50 by a const value somewhere, or change the criteria entirely
    }
}

enum QTNode
{
    Parent(QTParentNode),
    Leaf(QTLeafNode),
}

impl QTNode
{
    fn split(node: &mut QTNode, node_area: QTRect, node_depth: u8, max_depth: u8, entities_transforms: &HashMap<EntityId, Vec2>) -> Result<(), ()>
    {
        // takes a &mut QTNode and not a &mut QTLeafNode, because at the end, we set *node to a QTParentNode

        // TODO : errors that actually tells what happened (instead of returning Err(()) )

        let QTNode::Leaf(old_leaf) = node
        else
        {
            error!("trying to split a node that is already splitted");
            return Err(());
        };

        if node_depth >= max_depth
        {
            error!("trying to split a node that is at max depth");
            return Err(());
        }

        #[allow(irrefutable_let_patterns)]
        let new_shards: [ShardId; 4] = [0,0,0,0] // TODO : request to the orchestrator for 4 new shards (or have constantly a buffer of 4 available shards, so this call isn't async ?)
        else
        {
            error!("trying to split, but couldn't get 4 new shards");
            return Err(());
        };

        let mut new_children: [QTLeafNode; QTDir::NUMBER_OF_DIRS] = Default::default();

        // for now, each new child will keep the same shard_id as the node being split.
        // it will change only when the related entities have acccomplished an authority switch.
        for i in 0..QTDir::NUMBER_OF_DIRS
        {
            new_children[i].shard_id = old_leaf.shard_id;
        }

        // sort entities between the new children
        for entity_id in old_leaf.entities.iter()
        {
            let Some(entity_transform) = entities_transforms.get(&entity_id)
            else
            {
                error!("split() : entities_transforms doesn't have the transform for an entity that is inside the tree");
                return Err(());
            };

            let qt_dir = node_area.get_dir_from_pos(*entity_transform);
            
            new_children[qt_dir.as_usize()].entities.push(*entity_id);
        }


        // set on which shard each child will be
        let mut new_children_shard: [ShardId; QTDir::NUMBER_OF_DIRS] = Default::default();
        {   
            /*
            // old code from when there were only 3 new shards, instead of 4 :

            // the leaf that has the most amout of entities keeps the same shard as before the split, so there is less authority switch

            let mut max_entities_id: usize = 0;
            let mut max_entities_nb: usize = new_children[max_entities_id].entities.len();
            for curr_id in 1 .. QTDir::NUMBER_OF_DIRS
            {
                let curr_entities_nb = new_children[curr_id].entities.len();
                if curr_entities_nb > max_entities_nb
                {
                    max_entities_id = curr_id;
                    max_entities_nb = curr_entities_nb;
                }
            }
            // new_children[max_entities_id] == the child that has the most entities (between all nodes in new_children[], not the whole tree)

            let mut i_new_shards: usize = 0;
            for curr_id in 0 .. QTDir::NUMBER_OF_DIRS
            {
                if curr_id == max_entities_id
                {
                    new_children_shard[curr_id] = old_leaf.shard_id;
                }
                else
                {
                    new_children_shard[curr_id] = new_shards[i_new_shards];
                    i_new_shards += 1;
                }
            }

            */

            // new code for when there is 4 new shards, instead of 3 :
            for i in 0..QTDir::NUMBER_OF_DIRS
            {
                new_children_shard[i] = new_shards[i];
            }

        }
        // all nodes of new_children[] have been assigned a new shard
        
        let [child_ne, child_nw, child_sw, child_se] = new_children;
        // if we don't destructure the whole array in a single "instruction" (a rust instruction, not an ASM instruction),
        // we won't be able to move values out of the local array new_children.

        let new_node = QTParentNode {
            children: Box::new( [
                    QTNode::Leaf(child_ne),
                    QTNode::Leaf(child_nw),
                    QTNode::Leaf(child_sw),
                    QTNode::Leaf(child_se),
            ]),
        };

        *node = QTNode::Parent(new_node);

        // In the QuadTree, before this next function is called, "node" was split into 4 new leafs, but they all share the same shard_id, that is the shard_id of the original node.
        // We need to signal through the pub-sub that the entities of node->child[i] should start switching authority from the original shard id to new_children_shard[i].
        // When all entities are ready, the authority switch should happen, and the quad-tree should receive an event to correctly assign the leaf shard_id to the new shard id new_children_shard[i].
        return start_authority_switch_after_split(node, &new_children_shard);
    }

    #[allow(unused_variables)]
    fn start_authority_switch_after_split(node: &mut QTNode, new_shards: &[ShardId; QTDir::NUMBER_OF_DIRS]) -> Result<(), ()>
    {
        let QTNode::Parent(parent) = &node
        else
        {
            error!("start_authority_switch_after_split() : node isn't a parent node");
            return Err(());
        };

        for i in 0..QTDir::NUMBER_OF_DIRS
        {
            let QTNode::Leaf(leaf) = &(parent.children[i])
            else
            {
                error!("start_authority_switch_after_split() : parent.children[i] isn't a leaf node");
                return Err(());
            };

            for entity in leaf.entities.iter()
            {
                // TODO : publish to pub-sub : start switching authority of "entity" from "leaf.shard_id" to new_shards[i]
            }
        }

        // TODO : receive (probably by subscribing through the pub-sub to some topic) some message that tells that each entity of a shard is ready to switch authority
        //        then actually switch the authority of the entity AND do : leaf.shard_id = new_shards[i];
        // need to use game-sockets, ../shared for the GameMessages enum (to communicate with the pub-sub),
        // probably needs tokio too, to simplify async operations (like binding some function to the "all_entities_are_ready_to_switch_authority event")

        return Ok(());
    }

    fn get_node_that_contains_rec(node: &mut Self, node_area: QTRect, pos: Vec2) -> &mut QTLeafNode
    {
        match node
        {
            QTNode::Leaf(leaf) => { return leaf; }

            QTNode::Parent(parent) =>
            {
                let dir: QTDir = node_area.get_dir_from_pos(pos);
                let child_area: QTRect = node_area.get_quarter_from_dir(dir);
                return Self::get_node_that_contains_rec(&mut(parent.children[dir.as_usize()]), child_area, pos);
            }
        }
    }
    fn get_area_that_contains_rec(node: &Self, node_area: QTRect, pos: Vec2) -> QTRect
    {
        match node
        {
            QTNode::Leaf(_leaf) => { return node_area; }

            QTNode::Parent(parent) =>
            {
                let dir: QTDir = node_area.get_dir_from_pos(pos);
                let child_area: QTRect = node_area.get_quarter_from_dir(dir);
                return Self::get_area_that_contains_rec(&parent.children[dir.as_usize()], child_area, pos);
            }
        }
    }

    fn add_entity(node: &mut QTNode, node_area: QTRect, node_depth: u8, max_depth: u8, entities_transforms: &HashMap<EntityId, Vec2>, entity_id: EntityId, entity_transform: Vec2) -> Result<(),()>
    {
        if let QTNode::Leaf(leaf) = node
        {
            if leaf.should_split()
            {
                match Self::split(
                    node, 
                    node_area.clone(), 
                    node_depth, 
                    max_depth, 
                    entities_transforms // this is the only reason we need "entities_transforms: &HashMap<EntityId, Vec2>" as a parameter for the add_entity() function
                )
                {
                    Err(_err) => { return Err(_err); }
                    Ok(_) => {}
                }
            }

            // We need to recheck in case curr_node was split
            if let QTNode::Leaf(leaf) = node
            {
                leaf.entities.push(entity_id);
                return Ok(());
            }
        }

        // either parent from the beginning, or was children but was splitted, so became parent

        let QTNode::Parent(parent) = node
        else
        {
            panic!("add_entity: unreachable code reached");
        };

        let dir = node_area.get_dir_from_pos(entity_transform);
        let child_area: QTRect = node_area.get_quarter_from_dir(dir);
        
        return Self::add_entity(
            &mut (parent.children[dir.as_usize()]),
            child_area, 
            node_depth + 1,
            max_depth,
            entities_transforms,
            entity_id,
            entity_transform
        );
    }

}


pub struct QuadTree
{
    max_depth: u8,
    area: QTRect,
    entities_transforms: HashMap<EntityId, Vec2>,
    root: QTNode,
}

impl QuadTree
{
    pub fn new(max_depth: u8, area: QTRect, default_shard: ShardId) -> QuadTree
    {
        return QuadTree {
            max_depth: max_depth,
            area: area,
            root: QTNode::Leaf(QTLeafNode{
                shard_id: default_shard,
                ..Default::default()
            }),
            entities_transforms: HashMap::default(),
        };
    }

    pub fn add_entity(&mut self, entity_id: EntityId, entity_transform: Vec2) -> Result<(),()>
    {
        // entity_id isn't in both the quad tree and the hash map self.entities_transform,

        if self.entities_transforms.contains_key(&entity_id)
        {
            error!("add_entity() : trying to add an entity that is already in the quad_tree (entity {})", entity_id);
            return Err(());
        }

        self.entities_transforms.insert(entity_id.clone(), entity_transform.clone());

        return QTNode::add_entity(
            &mut self.root, 
            self.area.clone(), 
            0_u8, 
            self.max_depth, 
            &self.entities_transforms, 
            entity_id, 
            entity_transform
        );
    }

    pub fn update_entity_node(&mut self, entity_id: EntityId, entity_transform: Vec2) -> Result<(), ()>
    {
        // entity_id is removed from its old node, and is re-added into its new node
        let insert_result = self.entities_transforms.insert(entity_id.clone(), entity_transform.clone());

        let Some(old_transform) = insert_result
        else
        {
            error!("update_entity_node() : entity wasn't already in the quad_tree (entity {})", entity_id);
            return Err(());
        };

        // TODO : remove entity (so need to do a reverse-split function (merge) if merge criteria is met for parent node)
        let old_transform_node = QTNode::get_node_that_contains_rec(&mut self.root, self.area.clone(), old_transform);
        let Some(found_id) = old_transform_node.entities.iter().position(|val| {*val == entity_id} )
        else
        {
            panic!("update_entity_node() : entity {} had a transform in entities_transform, but was not in the corresponding leaf node", entity_id);
        };
        old_transform_node.entities.swap_remove(found_id);


        return self.add_entity(entity_id, entity_transform);
    }

    // returns the shard_id of the leaf containing "pos"
    pub fn get_shard_for_pos(&mut self, pos: Vec2) -> Option<ShardId>
    {
        if self.area.contains(pos) == false
        {
            error!("shard_for() : {} is outside the quad_tree area ({:?})", pos, self.area);
            return None;
        }
        let leaf = QTNode::get_node_that_contains_rec(&mut self.root, self.area.clone(), pos);
        return Some(leaf.shard_id);
    }
    
    /// Returns a list of shard_ids that overlaps with a circle of center "pos" and radius "margin"
    pub fn shards_near(&self, _pos: Vec2, _margin: f32) -> Vec<u32> 
    {
        return vec![];
    }
}






