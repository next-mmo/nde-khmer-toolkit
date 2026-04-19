use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static VOCABS: Lazy<HashMap<&'static str, usize>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(".", 0);
    m.insert("a", 1);
    m.insert("c", 2);
    m.insert("e", 3);
    m.insert("f", 4);
    m.insert("g", 5);
    m.insert("h", 6);
    m.insert("i", 7);
    m.insert("j", 8);
    m.insert("k", 9);
    m.insert("l", 10);
    m.insert("m", 11);
    m.insert("n", 12);
    m.insert("o", 13);
    m.insert("p", 14);
    m.insert("r", 15);
    m.insert("s", 16);
    m.insert("t", 17);
    m.insert("u", 18);
    m.insert("w", 19);
    m.insert("z", 20);
    m.insert("\u{014b}", 21); // ŋ
    m.insert("\u{0251}", 22); // ɑ
    m.insert("\u{0253}", 23); // ɓ
    m.insert("\u{0254}", 24); // ɔ
    m.insert("\u{0257}", 25); // ɗ
    m.insert("\u{0259}", 26); // ə
    m.insert("\u{025b}", 27); // ɛ
    m.insert("\u{0268}", 28); // ɨ
    m.insert("\u{0272}", 29); // ɲ
    m.insert("\u{0294}", 30); // ʔ
    m.insert("|", 31);
    m.insert("[UNK]", 32);
    m.insert("[PAD]", 33);
    m
});

pub fn get_vocab_id(token: &str) -> Option<usize> {
    VOCABS.get(token).copied()
}

pub fn time_to_frame(time: f64) -> usize {
    let stride_msec = 20.0;
    let frames_per_sec = 1000.0 / stride_msec;
    (time * frames_per_sec) as usize
}

pub fn intersperse<T: Clone>(lst: &[T], item: T) -> Vec<T> {
    let mut result = vec![item.clone(); lst.len() * 2 + 1];
    for (i, elem) in lst.iter().enumerate() {
        result[i * 2 + 1] = elem.clone();
    }
    result
}
