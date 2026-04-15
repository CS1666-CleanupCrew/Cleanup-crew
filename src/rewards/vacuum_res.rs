use crate::fluiddynamics::PulledByFluid;

pub const NAME: &str = "Vacuum Resistance";
pub const ASSET: &str = "rewards/HeartBox.png"; // TODO: replace with real sprite

pub fn apply(pull: &mut PulledByFluid) {
    // Heavier = harder to suck into breaches
    pull.mass += 25.0;
}
