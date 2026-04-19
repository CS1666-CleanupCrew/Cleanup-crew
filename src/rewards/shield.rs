use crate::player::Shield;

pub const NAME: &str = "Shield Charge";
pub const ASSET: &str = "rewards/Shield.png";

pub fn apply(shield: &mut Shield) {
    shield.max += 1.0;
    shield.current = (shield.current + 1.0).min(shield.max);
}
