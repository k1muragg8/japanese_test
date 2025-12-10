pub struct KanaPair {
    pub kana: &'static str,
    pub romaji: &'static str,
}

pub struct VocabPair {
    pub kanji: &'static str,
    pub kana: &'static str,
    pub meaning: &'static str,
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

pub const VOCAB_DATA: &[VocabPair] = &[
    VocabPair { kanji: "私", kana: "わたし", meaning: "I / Me", romaji: "watashi" },
    VocabPair { kanji: "猫", kana: "ねこ", meaning: "Cat", romaji: "neko" },
    VocabPair { kanji: "犬", kana: "いぬ", meaning: "Dog", romaji: "inu" },
    VocabPair { kanji: "食べる", kana: "たべる", meaning: "To eat", romaji: "taberu" },
    VocabPair { kanji: "見る", kana: "みる", meaning: "To see", romaji: "miru" },
    VocabPair { kanji: "本", kana: "ほん", meaning: "Book", romaji: "hon" },
    VocabPair { kanji: "水", kana: "みず", meaning: "Water", romaji: "mizu" },
    VocabPair { kanji: "行く", kana: "いく", meaning: "To go", romaji: "iku" },
    VocabPair { kanji: "山", kana: "やま", meaning: "Mountain", romaji: "yama" },
    VocabPair { kanji: "川", kana: "かわ", meaning: "River", romaji: "kawa" },
];
