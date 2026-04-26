use crate::weapons::Weapon;

pub const NAME: &str = "Piercing Rounds";
pub const ASSET: &str = "rewards/Piercing.png";

/// Each pickup adds one raw piercing stack.
/// Bullets use `Weapon::effective_pierce_count()` to determine the actual pierce level,
/// which applies diminishing returns after 4 pickups (2 pickups per +1 pierce level).
pub fn apply(weapon: &mut Weapon) {
    weapon.piercing_pickups += 1;
}
