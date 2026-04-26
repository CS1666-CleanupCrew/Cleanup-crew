use crate::weapons::Weapon;

pub const NAME: &str = "Damage Up";
pub const ASSET: &str = "rewards/DamageUp.png";

pub fn apply(weapon: &mut Weapon) {
    weapon.damage += 10.0;
}
