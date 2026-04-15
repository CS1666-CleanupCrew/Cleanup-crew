use rand::random_range;
use crate::player::{Health, MaxHealth};

pub const NAME: &str = "Max HP Up";
pub const ASSET: &str = "rewards/HeartBox.png";

pub fn apply(hp: &mut Health, maxhp: &mut MaxHealth) {
    let increase = random_range(5..=20) as f32;
    maxhp.0 += increase;
    hp.0 += increase;
}
