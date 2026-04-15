use crate::player::{MoveSpeed, ThrusterFuel};

pub const NAME: &str = "Move Speed Up";
pub const ASSET: &str = "rewards/MoveSpdBox.png";

pub fn apply(movspd: &mut MoveSpeed, fuel: &mut ThrusterFuel) {
    movspd.0 = (movspd.0 + 20.0).min(600.0);
    // Each Speed Up also extends the thruster fuel tank (max 10 charges)
    let new_max = (fuel.max + 3.0).min(10.0);
    let added = new_max - fuel.max;
    fuel.max = new_max;
    fuel.current = (fuel.current + added).min(fuel.max);
}
