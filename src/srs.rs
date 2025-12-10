use crate::models::UserProgress;
use chrono::{DateTime, Utc, Duration};

/// Calculates the next review interval and easiness factor based on the grade.
///
/// Grade:
/// 5 - Perfect response
/// 4 - Correct response after a hesitation
/// 3 - Correct response recalled with serious difficulty
/// 2 - Incorrect response; where the correct one seemed easy to recall
/// 1 - Incorrect response; the correct one remembered
/// 0 - Complete blackout.
///
/// For this simplified app:
/// Grade 5: First try correct.
/// Grade 0: Incorrect.
/// We can add intermediate grades later if UI supports "Hard/Good/Easy" buttons.
/// For now, we will stick to 5 (pass) and 0 (fail) logic, or maybe 5 and 3.
pub fn calculate_next_review(current_progress: &UserProgress, grade: u8) -> UserProgress {
    let mut progress = current_progress.clone();

    if grade >= 3 {
        // Correct
        if progress.repetitions == 0 {
            progress.interval = 1;
        } else if progress.repetitions == 1 {
            progress.interval = 6;
        } else {
            progress.interval = (progress.interval as f64 * progress.easiness_factor).round() as u64;
        }
        progress.repetitions += 1;
    } else {
        // Incorrect
        progress.repetitions = 0;
        progress.interval = 1;
    }

    // Update Easiness Factor (SM-2 formula)
    // EF' = EF + (0.1 - (5 - q) * (0.08 + (5 - q) * 0.02))
    // q = grade
    let q = grade as f64;
    let new_ef = progress.easiness_factor + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02));

    // EF should not drop below 1.3
    progress.easiness_factor = new_ef.max(1.3);

    // Update next review date
    progress.next_review_date = Utc::now() + Duration::days(progress.interval as i64);

    progress
}
