use std::time::Duration;
use crate::weapon::Weapon;

pub const NAME: &str = "Attack Speed Up";
pub const ASSET: &str = "rewards/AtkSpdBox.png";

pub fn apply(weapon: &mut Weapon) {
    let new_rate = (weapon.fire_rate - 0.03).max(0.1);
    weapon.fire_rate = new_rate;
    weapon.shoot_timer.set_duration(Duration::from_secs_f32(new_rate));
}
