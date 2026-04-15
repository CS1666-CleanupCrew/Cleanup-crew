use crate::player::Armor;

pub const NAME: &str = "Armor Up";
pub const ASSET: &str = "rewards/ArmorBox.png";

pub fn apply(armor: &mut Armor) {
    armor.0 += 20.0;
}
