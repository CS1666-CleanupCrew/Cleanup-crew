use crate::player::Regen;

pub const NAME: &str = "Regen";
pub const ASSET: &str = "rewards/HealthRegen.png";

pub fn apply(regen: &mut Regen) {
    regen.0 += 2.0;
}
