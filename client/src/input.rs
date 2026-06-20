use bevy::{
    prelude::*,
    input::keyboard::{
        Key,
        KeyCode,
    }
};

use shared::input::PlayerAction;
use crate::player::{
    MessagePlayerAction
};

// -------------------------------------------------------------------------------------------------------------------

// https://github.com/bevyengine/bevy/tree/latest/examples/input
#[allow(unused_variables)]
pub fn system_input_keyboard(keycodes_input: Res<ButtonInput<KeyCode>>, keys_input: Res<ButtonInput<Key>>, 
    mut msg_writer : MessageWriter<MessagePlayerAction> )
{
    for action in &PlayerAction::ALL
    {
        if keycodes_input.pressed(action.get_key_code()) // TODO : the Action->KeyCode should be a resource
        {
            // info!("{:?} (action {:?}) currently pressed", action.get_key_code(), action);
            msg_writer.write(MessagePlayerAction{act: *action});
        }
    }
}
