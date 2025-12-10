use crate::models::UserProgress;
use chrono::{Duration, Utc};

// SuperMemo-2 Algorithm
// https://en.wikipedia.org/wiki/SuperMemo#Description_of_SM-2_algorithm

pub fn calculate_new_progress(current: &UserProgress, grade: u8) -> UserProgress {
    let mut new_progress = current.clone();

    // Grade: 0-5.
    // In our quiz:
    // Incorrect -> 0 (or 1?)
    // Correct -> 4 or 5 depending on speed?
    // For now, let's assume the caller maps the quiz result to a grade.
    // If we only have correct/incorrect:
    // Correct -> 5 (Perfect response)
    // Incorrect -> 0 (Complete blackout)

    if grade >= 3 {
        if new_progress.repetitions == 0 {
            new_progress.interval = 1;
        } else if new_progress.repetitions == 1 {
            new_progress.interval = 6;
        } else {
            new_progress.interval = (new_progress.interval as f64 * new_progress.easiness_factor).round() as u64;
        }
        new_progress.repetitions += 1;
    } else {
        new_progress.repetitions = 0;
        new_progress.interval = 1;
    }

    new_progress.easiness_factor = new_progress.easiness_factor + (0.1 - (5.0 - grade as f64) * (0.08 + (5.0 - grade as f64) * 0.02));
    if new_progress.easiness_factor < 1.3 {
        new_progress.easiness_factor = 1.3;
    }

    new_progress.next_review_date = Utc::now() + Duration::days(new_progress.interval as i64);

    new_progress
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sm2_correct_first_time() {
        let p = UserProgress::new("a".to_string());
        let new_p = calculate_new_progress(&p, 5);
        assert_eq!(new_p.interval, 1);
        assert_eq!(new_p.repetitions, 1);
        assert!(new_p.easiness_factor > 2.5); // 2.5 + 0.1 = 2.6
    }

    #[test]
    fn test_sm2_incorrect() {
        let mut p = UserProgress::new("a".to_string());
        p.interval = 10;
        p.repetitions = 5;

        let new_p = calculate_new_progress(&p, 0); // Forgot
        assert_eq!(new_p.interval, 1);
        assert_eq!(new_p.repetitions, 0);
        assert!(new_p.easiness_factor < 2.5);
    }
}
