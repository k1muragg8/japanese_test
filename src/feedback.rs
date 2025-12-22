use crate::data::KANA_DATA;

#[allow(unused)]
pub struct FeedbackGenerator;

#[allow(unused)]
impl FeedbackGenerator {
    pub fn generate_explanation(correct_kana: &str, correct_romaji: &str, user_input: &str) -> String {
        let trimmed_input = user_input.trim();

        // Find if user input matches another kana
        let confused_kana = KANA_DATA.iter().find(|k| k.romaji == trimmed_input);

        let mut msg = format!(
            "正确答案是 {} ({})。 你输入了: '{}'。 请继续加油！",
            correct_kana, correct_romaji, trimmed_input
        );

        if let Some(confused) = confused_kana {
            if confused.kana != correct_kana {
                msg.push_str(&format!("\n你输入的 '{}' 对应的假名是 '{}'。", trimmed_input, confused.kana));
            }
        }

        msg
    }
}
