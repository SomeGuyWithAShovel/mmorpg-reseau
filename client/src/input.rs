use bevy::{
    prelude::*,
    input::keyboard::{
        Key,
        KeyCode,
        KeyboardInput,
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

// -------------------------------------------------------------------------------------------------------------------

#[allow(dead_code)] // unused : see comments
#[deprecated(note = "use system_input_keyboard() instead")]
pub fn messages_input_keyboard(mut keyboard_inputs: MessageReader<KeyboardInput>)
{
    for keyboard_input in keyboard_inputs.read()
    {
        info!("{:?}", keyboard_input);
        // when we press a key, we have : 
        // "k            kkkkkkkkkkkkkkkkkkkkkkkkkkk", like when we write text.
        // whereas in the system input version, we have the wanted behaviour.
    }
}

// -------------------------------------------------------------------------------------------------------------------
