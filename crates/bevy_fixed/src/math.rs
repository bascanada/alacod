use bevy::math::{Vec2, Vec3};

use crate::{fixed_math, rng::RollbackRng};


pub fn calculate_spread_angle(
    rng: &mut RollbackRng,
    spread: fixed_math::Fixed,
) -> fixed_math::Fixed {
    // Original line:
    // let spread_angle = (rand::random::<f32>() - 0.5)  * .spread;

    // New rollback-appropriate line:
    // Generate a random f32 between 0.0 and 1.0
    let random_val_0_to_1 = rng.next_fixed();

    // Convert to a range of -0.5 to 0.5 (similar to original logic)
    let random_val_neg_0_5_to_0_5 = random_val_0_to_1 - fixed_math::new(0.5);

    let spread_angle = random_val_neg_0_5_to_0_5 * spread;
    spread_angle
}

// UI FUNCTION
pub fn calculate_time_remaining_seconds(ending_frame_number: u32, current_frame: u32) -> f32 {
    // Ensure current_frame does not exceed ending_frame_number to prevent underflow
    // and negative time. If it does, no time is remaining.
    if current_frame >= ending_frame_number {
        return 0.0;
    }

    // Calculate the number of frames remaining
    let frames_remaining: u32 = ending_frame_number - current_frame;

    // Convert frames remaining to seconds
    // We cast frames_remaining to f32 for the division
    let time_remaining_seconds: f32 = frames_remaining as f32 / 60.0;

    time_remaining_seconds
}

#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (RollbackRng)
    use std::f32::consts::PI; // For testing spread angle

    #[test]
    fn test_calculate_spread_angle_range() {
        let mut rng = RollbackRng::new(88888);
        let max_spread_rad = PI / 4.0; // e.g., 45 degrees max spread in one direction from center

        for _ in 0..1000 {
            let angle = calculate_spread_angle(&mut rng, max_spread_rad);
            // The angle should be between -max_spread_rad / 2 and +max_spread_rad / 2
            // if random_val_neg_0_5_to_0_5 is in [-0.5, 0.5)
            // So, angle is in [-max_spread_rad * 0.5, max_spread_rad * 0.5)
            let half_max_spread = max_spread_rad * 0.5;
            assert!(angle >= -half_max_spread && angle < half_max_spread,
                    "Calculated spread angle {} was not in range [{}, {}) for max_spread_rad {}",
                    angle, -half_max_spread, half_max_spread, max_spread_rad);
        }

        // Test with zero spread
        let angle_zero_spread = calculate_spread_angle(&mut rng, 0.0);
        assert_eq!(angle_zero_spread, 0.0, "Angle with zero spread should be zero.");
    }
}