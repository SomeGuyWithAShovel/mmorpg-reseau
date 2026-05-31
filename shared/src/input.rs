use bevy::prelude::*;

// #[allow(unused)]
#[derive(Debug, Clone, Copy)] // Debug so it can be printed with {:?}
pub enum PlayerAction
{
    Forward,
    Backward,
    Left,
    Right,
    Boost,
    Shoot,
    Dodge,
    Extra
}

impl PlayerAction 
{  
    pub const ALL: [Self; 8] = [ // NO WARNINGS WHEN ADDING A NEW VALUE
        PlayerAction::Forward,
        PlayerAction::Backward,
        PlayerAction::Left,
        PlayerAction::Right,
        PlayerAction::Boost,
        PlayerAction::Shoot,
        PlayerAction::Dodge,
        PlayerAction::Extra
    ];

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

#[derive(Component, Default)] // Default, so it can initialize itself using u8::default()
pub struct PlayerActionHolder
{
    data: u8,
}

impl PlayerActionHolder // so I don't start to do bitwise operations everywhere
{
    pub fn get_from_act(act: PlayerAction) -> u8
    {
        match act
        {
            PlayerAction::Forward  => { return 0x01_u8; }
            PlayerAction::Backward => { return 0x02_u8; }
            PlayerAction::Left     => { return 0x04_u8; }
            PlayerAction::Right    => { return 0x08_u8; }
            PlayerAction::Boost    => { return 0x10_u8; }
            PlayerAction::Shoot    => { return 0x20_u8; }
            PlayerAction::Dodge    => { return 0x40_u8; }
            PlayerAction::Extra    => { return 0x80_u8; }
        }
    }
    
    // ---------------------------------------------------------------------------------------------------------------

    pub fn clear_acts(&mut self) { self.data = 0u8; }

    pub fn add_act(&mut self, act: PlayerAction)
    {
        self.data |= Self::get_from_act(act);
    }
    
    // ---------------------------------------------------------------------------------------------------------------
    
    pub fn check_act(&self, act: PlayerAction) -> bool
    {
        return (self.data & Self::get_from_act(act)) != 0u8;
    }
    
    // ---------------------------------------------------------------------------------------------------------------
    
    pub fn get_move_dir(&self) -> Vec2
    {
        let mut move_dir = Vec2 {x: 0.0, y: 0.0};

        if self.check_act(PlayerAction::Forward)
        {
            move_dir.y +=  1.0;    
        }
        if self.check_act(PlayerAction::Backward)
        {
            move_dir.y += -1.0;
        }
        if self.check_act(PlayerAction::Left)
        {
            move_dir.x += -1.0;
        }
        if self.check_act(PlayerAction::Right)
        {
            move_dir.x +=  1.0;
        }
        return move_dir.normalize();
    }
    
    // ---------------------------------------------------------------------------------------------------------------

}
