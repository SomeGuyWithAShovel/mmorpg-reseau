use bevy::{
    prelude::*,
    input::keyboard::{
        Key,
        KeyCode,
        KeyboardInput,
    }
};

use crate::player::{
    PlayerAction,
    MessagePlayerAction
};

// -------------------------------------------------------------------------------------------------------------------

impl PlayerAction
{
    #[must_use] // [nodiscard]
    pub const fn get_key_code(&self) -> KeyCode
    {
        match self
        {
            PlayerAction::Forward  => { return KeyCode::KeyW; }
            PlayerAction::Backward => { return KeyCode::KeyS; }
            PlayerAction::Left     => { return KeyCode::KeyA; }
            PlayerAction::Right    => { return KeyCode::KeyD; }
            PlayerAction::Boost    => { return KeyCode::ShiftLeft; }
            PlayerAction::Shoot    => { return KeyCode::Space; }
            PlayerAction::Dodge    => { return KeyCode::ControlLeft; }
            PlayerAction::Extra    => { return KeyCode::KeyE; }
        }
    }
}

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
