use crate::player::AirTank;

pub const NAME: &str = "Slower Air Drain";
pub const ASSET: &str = "rewards/DrainRate.png";

pub fn apply(tank: &mut AirTank) {
    // 20% reduction per pickup, minimum 0.2 units/sec
    tank.drain_rate = (tank.drain_rate * 0.8).max(0.2);
}
