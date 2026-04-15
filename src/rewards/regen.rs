use crate::player::Regen;

pub const NAME: &str = "Regen";
pub const ASSET: &str = "rewards/HeartBox.png"; // TODO: replace with real sprite

pub fn apply(regen: &mut Regen) {
    regen.0 += 2.0;
}
