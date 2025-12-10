use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kana {
    pub character: &'static str,
    pub romaji: &'static str,
}

pub const KANA_DATA: &[Kana] = &[
    // Hiragana - Seion
    Kana { character: "あ", romaji: "a" }, Kana { character: "い", romaji: "i" }, Kana { character: "う", romaji: "u" }, Kana { character: "え", romaji: "e" }, Kana { character: "お", romaji: "o" },
    Kana { character: "か", romaji: "ka" }, Kana { character: "き", romaji: "ki" }, Kana { character: "く", romaji: "ku" }, Kana { character: "け", romaji: "ke" }, Kana { character: "こ", romaji: "ko" },
    Kana { character: "さ", romaji: "sa" }, Kana { character: "し", romaji: "shi" }, Kana { character: "す", romaji: "su" }, Kana { character: "せ", romaji: "se" }, Kana { character: "そ", romaji: "so" },
    Kana { character: "た", romaji: "ta" }, Kana { character: "ち", romaji: "chi" }, Kana { character: "つ", romaji: "tsu" }, Kana { character: "て", romaji: "te" }, Kana { character: "と", romaji: "to" },
    Kana { character: "な", romaji: "na" }, Kana { character: "に", romaji: "ni" }, Kana { character: "ぬ", romaji: "nu" }, Kana { character: "ね", romaji: "ne" }, Kana { character: "の", romaji: "no" },
    Kana { character: "は", romaji: "ha" }, Kana { character: "ひ", romaji: "hi" }, Kana { character: "ふ", romaji: "fu" }, Kana { character: "へ", romaji: "he" }, Kana { character: "ほ", romaji: "ho" },
    Kana { character: "ま", romaji: "ma" }, Kana { character: "み", romaji: "mi" }, Kana { character: "む", romaji: "mu" }, Kana { character: "め", romaji: "me" }, Kana { character: "も", romaji: "mo" },
    Kana { character: "や", romaji: "ya" }, Kana { character: "ゆ", romaji: "yu" }, Kana { character: "よ", romaji: "yo" },
    Kana { character: "ら", romaji: "ra" }, Kana { character: "り", romaji: "ri" }, Kana { character: "る", romaji: "ru" }, Kana { character: "れ", romaji: "re" }, Kana { character: "ろ", romaji: "ro" },
    Kana { character: "わ", romaji: "wa" }, Kana { character: "を", romaji: "wo" }, Kana { character: "ん", romaji: "n" },

    // Hiragana - Dakuon
    Kana { character: "が", romaji: "ga" }, Kana { character: "ぎ", romaji: "gi" }, Kana { character: "ぐ", romaji: "gu" }, Kana { character: "げ", romaji: "ge" }, Kana { character: "ご", romaji: "go" },
    Kana { character: "ざ", romaji: "za" }, Kana { character: "じ", romaji: "ji" }, Kana { character: "ず", romaji: "zu" }, Kana { character: "ぜ", romaji: "ze" }, Kana { character: "ぞ", romaji: "zo" },
    Kana { character: "だ", romaji: "da" }, Kana { character: "ぢ", romaji: "ji" }, Kana { character: "づ", romaji: "zu" }, Kana { character: "で", romaji: "de" }, Kana { character: "ど", romaji: "do" },
    Kana { character: "ば", romaji: "ba" }, Kana { character: "び", romaji: "bi" }, Kana { character: "ぶ", romaji: "bu" }, Kana { character: "べ", romaji: "be" }, Kana { character: "ぼ", romaji: "bo" },
    Kana { character: "ぱ", romaji: "pa" }, Kana { character: "ぴ", romaji: "pi" }, Kana { character: "ぷ", romaji: "pu" }, Kana { character: "ぺ", romaji: "pe" }, Kana { character: "ぽ", romaji: "po" },

    // Katakana - Seion
    Kana { character: "ア", romaji: "a" }, Kana { character: "イ", romaji: "i" }, Kana { character: "ウ", romaji: "u" }, Kana { character: "エ", romaji: "e" }, Kana { character: "オ", romaji: "o" },
    Kana { character: "カ", romaji: "ka" }, Kana { character: "キ", romaji: "ki" }, Kana { character: "ク", romaji: "ku" }, Kana { character: "ケ", romaji: "ke" }, Kana { character: "コ", romaji: "ko" },
    Kana { character: "サ", romaji: "sa" }, Kana { character: "シ", romaji: "shi" }, Kana { character: "ス", romaji: "su" }, Kana { character: "セ", romaji: "se" }, Kana { character: "ソ", romaji: "so" },
    Kana { character: "タ", romaji: "ta" }, Kana { character: "チ", romaji: "chi" }, Kana { character: "ツ", romaji: "tsu" }, Kana { character: "テ", romaji: "te" }, Kana { character: "ト", romaji: "to" },
    Kana { character: "ナ", romaji: "na" }, Kana { character: "ニ", romaji: "ni" }, Kana { character: "ヌ", romaji: "nu" }, Kana { character: "ネ", romaji: "ne" }, Kana { character: "ノ", romaji: "no" },
    Kana { character: "ハ", romaji: "ha" }, Kana { character: "ヒ", romaji: "hi" }, Kana { character: "フ", romaji: "fu" }, Kana { character: "ヘ", romaji: "he" }, Kana { character: "ホ", romaji: "ho" },
    Kana { character: "マ", romaji: "ma" }, Kana { character: "ミ", romaji: "mi" }, Kana { character: "ム", romaji: "mu" }, Kana { character: "メ", romaji: "me" }, Kana { character: "モ", romaji: "mo" },
    Kana { character: "ヤ", romaji: "ya" }, Kana { character: "ユ", romaji: "yu" }, Kana { character: "ヨ", romaji: "yo" },
    Kana { character: "ラ", romaji: "ra" }, Kana { character: "リ", romaji: "ri" }, Kana { character: "ル", romaji: "ru" }, Kana { character: "レ", romaji: "re" }, Kana { character: "ロ", romaji: "ro" },
    Kana { character: "ワ", romaji: "wa" }, Kana { character: "ヲ", romaji: "wo" }, Kana { character: "ン", romaji: "n" },

    // Katakana - Dakuon
    Kana { character: "ガ", romaji: "ga" }, Kana { character: "ギ", romaji: "gi" }, Kana { character: "グ", romaji: "gu" }, Kana { character: "ゲ", romaji: "ge" }, Kana { character: "ゴ", romaji: "go" },
    Kana { character: "ザ", romaji: "za" }, Kana { character: "ジ", romaji: "ji" }, Kana { character: "ズ", romaji: "zu" }, Kana { character: "ゼ", romaji: "ze" }, Kana { character: "ゾ", romaji: "zo" },
    Kana { character: "ダ", romaji: "da" }, Kana { character: "ヂ", romaji: "ji" }, Kana { character: "ヅ", romaji: "zu" }, Kana { character: "デ", romaji: "de" }, Kana { character: "ド", romaji: "do" },
    Kana { character: "バ", romaji: "ba" }, Kana { character: "ビ", romaji: "bi" }, Kana { character: "ブ", romaji: "bu" }, Kana { character: "ベ", romaji: "be" }, Kana { character: "ボ", romaji: "bo" },
    Kana { character: "パ", romaji: "pa" }, Kana { character: "ピ", romaji: "pi" }, Kana { character: "プ", romaji: "pu" }, Kana { character: "ペ", romaji: "pe" }, Kana { character: "ポ", romaji: "po" },
];
