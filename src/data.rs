pub struct KanaPair {
    pub kana: &'static str,
    pub romaji: &'static str,
}

pub const KANA_DATA: &[KanaPair] = &[
    KanaPair { kana: "あ", romaji: "a" },
    KanaPair { kana: "い", romaji: "i" },
    KanaPair { kana: "う", romaji: "u" },
    KanaPair { kana: "え", romaji: "e" },
    KanaPair { kana: "お", romaji: "o" },
    KanaPair { kana: "か", romaji: "ka" },
    KanaPair { kana: "き", romaji: "ki" },
    KanaPair { kana: "く", romaji: "ku" },
    KanaPair { kana: "け", romaji: "ke" },
    KanaPair { kana: "こ", romaji: "ko" },
    KanaPair { kana: "さ", romaji: "sa" },
    KanaPair { kana: "し", romaji: "shi" },
    KanaPair { kana: "す", romaji: "su" },
    KanaPair { kana: "せ", romaji: "se" },
    KanaPair { kana: "そ", romaji: "so" },
    KanaPair { kana: "ア", romaji: "a" },
    KanaPair { kana: "イ", romaji: "i" },
    KanaPair { kana: "ウ", romaji: "u" },
    KanaPair { kana: "エ", romaji: "e" },
    KanaPair { kana: "オ", romaji: "o" },
];

pub struct VocabItem {
    pub kanji: Option<&'static str>,
    pub kana: &'static str,
    pub meaning: &'static str,
    pub romaji: &'static str,
}

pub const VOCAB_DATA: &[VocabItem] = &[
    VocabItem { kanji: Some("私"), kana: "わたし", meaning: "I, me", romaji: "watashi" },
    VocabItem { kanji: Some("猫"), kana: "ねこ", meaning: "Cat", romaji: "neko" },
    VocabItem { kanji: Some("犬"), kana: "いぬ", meaning: "Dog", romaji: "inu" },
    VocabItem { kanji: Some("食べる"), kana: "たべる", meaning: "To eat", romaji: "taberu" },
    VocabItem { kanji: Some("水"), kana: "みず", meaning: "Water", romaji: "mizu" },
    VocabItem { kanji: Some("本"), kana: "ほん", meaning: "Book", romaji: "hon" },
    VocabItem { kanji: Some("学生"), kana: "がくせい", meaning: "Student", romaji: "gakusei" },
    VocabItem { kanji: Some("先生"), kana: "せんせい", meaning: "Teacher", romaji: "sensei" },
    VocabItem { kanji: Some("学校"), kana: "がっこう", meaning: "School", romaji: "gakkou" },
    VocabItem { kanji: Some("行く"), kana: "いく", meaning: "To go", romaji: "iku" },
];
