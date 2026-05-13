mod common;
mod entity;
mod input;
mod player;
mod enemy;

// -------------------------------------------------------------------------------------------------------------------

use bevy::{
    prelude::*,
};

// -------------------------------------------------------------------------------------------------------------------

fn main()
{
    info!("main start");

    let mut app: App = App::new();
    
    app.add_plugins(

        // without that, we don't have a window and the app auto-closes after 1 tick.
        DefaultPlugins
            // sets the default image sampler mode to nearest, instead of linear, so we don't have to change it every single time we load an asset
            // .set(ImagePlugin { default_sampler: ImageSamplerDescriptor::nearest() })
    );

    app.add_systems(PreStartup, startup_print);
    app.add_systems(Last, exit_print); // Last fires each ticks (after all other "OnTick" schedules), so our system needs to have a MessageReader<AppExit> to run only on AppExit
    


    app.add_plugins(entity::EntityPlugin);
    app.add_plugins(player::PlayerPlugin);
    app.add_plugins(enemy::EnemyPlugin);
    
    

    app.run();

}

// -------------------------------------------------------------------------------------------------------------------

pub fn startup_print()
{
    println!("starting bevy application...");
}

// -------------------------------------------------------------------------------------------------------------------

// there's no "end of app" schedule, so we need to specify its parameter to read the Message AppExit, so this system is fired only when the message is emitted
pub fn exit_print(mut msgs: MessageReader<AppExit>)
{
    for _app_exit in msgs.read() // AppExit is a special message, so the loop isn't really needed, 
    // but to read a message more generally, we need to loop over them (since there might be multiple in one tick)
    {
        println!("exiting...");
    }
}
