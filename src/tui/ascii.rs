//! ASCII-фолбэк для терминалов без CJK/Unicode-глифов (legacy conhost/cmd на Windows).
//!
//! Когда включён, весь текст интерфейса преобразуется в чистый ASCII:
//! латиница как есть, кириллица — транслит, хангыль — романизация
//! (Revised Romanization), известные символы UI — замены, остальное — `?`.
//!
//! Автоопределение: Windows без признаков современного терминала (`WT_SESSION`,
//! `TERM_PROGRAM`, `WEZTERM_PANE`, `KITTY_WINDOW_ID`, `ALACRITTY_WINDOW_ID`,
//! `ConEmuPID`). Переопределение вручную: `KAKAO_ASCII=1` / `KAKAO_ASCII=0`
//! (работает и на Linux — для проверки).

use std::{ borrow::Cow, sync::LazyLock };

static ENABLED: LazyLock<bool> = LazyLock::new(detect);

pub fn enabled() -> bool {
    *ENABLED
}

/// Текст для отрисовки: в ASCII-режиме — транслит, иначе как есть (без аллокации).
pub fn convert(text: &str) -> Cow<'_, str> {
    if !enabled() {
        return Cow::Borrowed(text);
    }

    Cow::Owned(transliterate(text))
}

fn detect() -> bool {
    match std::env::var("KAKAO_ASCII").as_deref() {
        Ok(value) =>
            match value.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" | "force" => true,
                "0" | "false" | "no" | "off" => false,
                _ => auto_detect(),
            }
        Err(_) => auto_detect(),
    }
}

fn auto_detect() -> bool {
    #[cfg(windows)]
    {
        const MODERN: [&str; 6] = [
            "WT_SESSION",
            "TERM_PROGRAM",
            "WEZTERM_PANE",
            "KITTY_WINDOW_ID",
            "ALACRITTY_WINDOW_ID",
            "ConEmuPID",
        ];

        !MODERN.iter().any(|key| std::env::var_os(key).is_some())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Всегда преобразует в ASCII (чистая функция, без учёта `enabled()`).
pub fn transliterate(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
            continue;
        }

        match c {
            '…' => out.push_str("..."),
            '—' | '–' | '―' => out.push('-'),
            '«' | '»' | '“' | '”' | '„' => out.push('"'),
            '‘' | '’' => out.push('\''),
            '─' | '━' | '┄' | '┈' => out.push('-'),
            '│' | '┃' | '║' | '┆' | '┇' | '┊' | '┋' | '╎' | '╏' => out.push('|'),
            '▏' => out.push('|'),
            '·' | '•' | '∙' | '◆' | '◇' | '●' | '○' => out.push('*'),
            '✓' | '✔' => out.push('+'),
            '✕' | '✗' | '×' => out.push('x'),
            '←' => out.push('<'),
            '→' => out.push('>'),
            '↑' => out.push('^'),
            '↓' => out.push('v'),
            '‹' => out.push('<'),
            '›' => out.push('>'),
            '№' => out.push('N'),
            'Ａ'..='Ｚ' => out.push((c as u32 - 0xff21 + u32::from(b'A')) as u8 as char),
            'ａ'..='ｚ' => out.push((c as u32 - 0xff41 + u32::from(b'a')) as u8 as char),
            '０'..='９' => out.push((c as u32 - 0xff10 + u32::from(b'0')) as u8 as char),
            'Ё' => out.push_str("Yo"),
            'ё' => out.push_str("yo"),
            'Ж' => out.push_str("Zh"),
            'ж' => out.push_str("zh"),
            'Х' => out.push_str("Kh"),
            'х' => out.push_str("kh"),
            'Ц' => out.push_str("Ts"),
            'ц' => out.push_str("ts"),
            'Ч' => out.push_str("Ch"),
            'ч' => out.push_str("ch"),
            'Ш' => out.push_str("Sh"),
            'ш' => out.push_str("sh"),
            'Щ' => out.push_str("Shch"),
            'щ' => out.push_str("shch"),
            'Ю' => out.push_str("Yu"),
            'ю' => out.push_str("yu"),
            'Я' => out.push_str("Ya"),
            'я' => out.push_str("ya"),
            'А' => out.push('A'),
            'а' => out.push('a'),
            'Б' => out.push('B'),
            'б' => out.push('b'),
            'В' => out.push('V'),
            'в' => out.push('v'),
            'Г' => out.push('G'),
            'г' => out.push('g'),
            'Д' => out.push('D'),
            'д' => out.push('d'),
            'Е' => out.push('E'),
            'е' => out.push('e'),
            'З' => out.push('Z'),
            'з' => out.push('z'),
            'И' => out.push('I'),
            'и' => out.push('i'),
            'Й' => out.push('Y'),
            'й' => out.push('y'),
            'К' => out.push('K'),
            'к' => out.push('k'),
            'Л' => out.push('L'),
            'л' => out.push('l'),
            'М' => out.push('M'),
            'м' => out.push('m'),
            'Н' => out.push('N'),
            'н' => out.push('n'),
            'О' => out.push('O'),
            'о' => out.push('o'),
            'П' => out.push('P'),
            'п' => out.push('p'),
            'Р' => out.push('R'),
            'р' => out.push('r'),
            'С' => out.push('S'),
            'с' => out.push('s'),
            'Т' => out.push('T'),
            'т' => out.push('t'),
            'У' => out.push('U'),
            'у' => out.push('u'),
            'Ф' => out.push('F'),
            'ф' => out.push('f'),
            'Ы' => out.push('Y'),
            'ы' => out.push('y'),
            'Ь' => out.push('\''),
            'ь' => out.push('\''),
            'Э' => out.push('E'),
            'э' => out.push('e'),
            'Ъ' | 'ъ' => {}
            '\u{ac00}'..='\u{d7a3}' => push_hangul(&mut out, c),
            _ => out.push('?'),
        }
    }

    out
}

/// Один предсоставленный слог хангыля → Revised Romanization.
fn push_hangul(out: &mut String, c: char) {
    const LEAD: [&str; 19] = [
        "g",
        "kk",
        "n",
        "d",
        "tt",
        "r",
        "m",
        "b",
        "pp",
        "s",
        "ss",
        "",
        "j",
        "jj",
        "ch",
        "k",
        "t",
        "p",
        "h",
    ];
    const VOWEL: [&str; 21] = [
        "a",
        "ae",
        "ya",
        "yae",
        "eo",
        "e",
        "yeo",
        "ye",
        "o",
        "wa",
        "wae",
        "oe",
        "yo",
        "u",
        "wo",
        "we",
        "wi",
        "yu",
        "eu",
        "yi",
        "i",
    ];
    const TAIL: [&str; 28] = [
        "",
        "k",
        "k",
        "ks",
        "n",
        "nj",
        "nh",
        "d",
        "l",
        "lk",
        "lm",
        "lb",
        "ls",
        "lt",
        "lp",
        "lh",
        "m",
        "b",
        "bs",
        "s",
        "ss",
        "ng",
        "j",
        "ch",
        "k",
        "t",
        "p",
        "h",
    ];

    let mut syllable = c as u32 - 0xac00;
    let tail = (syllable % 28) as usize;
    syllable /= 28;
    let vowel = (syllable % 21) as usize;
    let lead = (syllable / 21) as usize;

    out.push_str(LEAD[lead]);
    out.push_str(VOWEL[vowel]);
    out.push_str(TAIL[tail]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangul_is_romanized() {
        assert_eq!(transliterate("한글"), "hangeul");
    }

    #[test]
    fn cyrillic_is_transliterated() {
        assert_eq!(transliterate("Главы"), "Glavy");
        assert_eq!(transliterate("Загрузка страниц"), "Zagruzka stranits");
    }

    #[test]
    fn ui_symbols_are_mapped() {
        assert_eq!(transliterate("✓ ◆ ✕"), "+ * x");
        assert_eq!(transliterate("←→ ↑↓"), "<> ^v");
        assert_eq!(transliterate("«тест» — …"), "\"test\" - ...");
    }

    #[test]
    fn everything_is_ascii() {
        for sample in [
            "한글 테스트 ✓",
            "Глава №1 «тест» — …",
            "поиск / название или номер главы",
            "тайтл 54801072 · глав 25 · стр 1/2",
        ] {
            let converted = transliterate(sample);
            assert!(
                converted.is_ascii(),
                "{sample} -> {converted}"
            );
        }
    }

    #[test]
    fn ascii_passes_through() {
        assert_eq!(transliterate("abc 123 / <id>"), "abc 123 / <id>");
    }
}
