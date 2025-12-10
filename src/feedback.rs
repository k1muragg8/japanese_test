use crate::data::get_all_kana;

pub struct FeedbackGenerator;

impl FeedbackGenerator {
    pub fn generate_explanation(correct_kana: &str, correct_romaji: &str, user_input: &str) -> String {
        let trimmed_input = user_input.trim();
        let all_kana = get_all_kana();

        // Find if user input matches another kana
        let confused_kana = all_kana.iter().find(|(_, romaji)| *romaji == trimmed_input);

        let mut msg = format!(
            "正确答案是 {} ({})。 你输入了: '{}'。 请继续加油！",
            correct_kana, correct_romaji, trimmed_input
        );

        if let Some((confused_char, _)) = confused_kana {
            if *confused_char != correct_kana {
                msg.push_str(&format!("\n你输入的 '{}' 对应的假名是 '{}'。", trimmed_input, confused_char));
            }
        }

        msg
    }
}
