use crate::player::AirTank;

pub const NAME: &str = "Larger Air Tank";
pub const ASSET: &str = "rewards/LargerTank.png";

pub fn apply(tank: &mut AirTank) {
    tank.max_capacity += 2.5;
    tank.current = (tank.current + 2.5).min(tank.max_capacity);
}
