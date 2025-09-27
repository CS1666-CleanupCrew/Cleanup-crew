use bevy::{prelude::*, window::PresentMode};

mod endcredits;


fn main() {
    endcredits::run_slideshow();
}

fn log_state_change(state: Res<State<GameState>>) {
    info!("Just moved to {:?}!", state.get());
}