use std::sync::Arc;

use reqwest::{ Url, cookie::{ CookieStore, Jar } };

use crate::{ config::Config, error::Result };

const COOKIE_ATTRIBUTES: [&str; 8] = [
    "domain",
    "path",
    "expires",
    "max-age",
    "samesite",
    "secure",
    "httponly",
    "partitioned",
];

pub fn load_into_jar(jar: &Jar, base_url: &Url, raw: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    for entry in raw.split([';', '\n', '\r']) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let Some((name, value)) = entry.split_once('=') else {
            warnings.push(format!("запись без '=' пропущена: {entry:?}"));
            continue;
        };

        let (name, value) = (name.trim(), value.trim());
        if is_attribute(name) {
            // В файле могут попадаться остатки атрибутов (`Domain=.kakao.com`, `Max-Age=7200`).
            continue;
        }
        if !is_valid_name(name) {
            warnings.push(format!("некорректное имя куки, запись пропущена: {entry:?}"));
            continue;
        }

        let set_cookie = format!("{name}={value}; Domain=.kakao.com; Path=/");
        jar.add_cookie_str(&set_cookie, base_url);
    }

    warnings
}

pub fn sync_to_file(jar: &Arc<Jar>, url: &Url) -> Result<()> {
    let cookie = jar
        .cookies(url)
        .and_then(|value| value.to_str().ok().map(str::to_owned))
        .unwrap_or_default();

    if cookie.trim().is_empty() {
        return Ok(());
    }

    let mut config = Config::load()?;
    config.cookie = cookie;
    config.save()
}

fn is_attribute(name: &str) -> bool {
    COOKIE_ATTRIBUTES.contains(&name.to_ascii_lowercase().as_str())
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty() &&
        name
            .chars()
            .all(|c| {
                c.is_ascii_alphanumeric() ||
                    matches!(
                        c,
                        '!' |
                            '#' |
                            '$' |
                            '%' |
                            '&' |
                            '\'' |
                            '*' |
                            '+' |
                            '-' |
                            '.' |
                            '^' |
                            '_' |
                            '`' |
                            '|' |
                            '~'
                    )
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kakao::url::parse_url;

    const BASE_URL: &str = "https://page.kakao.com";

    fn cookies_for(raw: &str, target: &str) -> String {
        let jar = Jar::default();
        let base_url = parse_url(BASE_URL).expect("корректный базовый URL");
        load_into_jar(&jar, &base_url, raw);

        let target = parse_url(target).expect("корректный целевой URL");
        jar.cookies(&target)
            .and_then(|value| value.to_str().ok().map(str::to_owned))
            .unwrap_or_default()
    }

    #[test]
    fn loads_every_cookie_not_only_the_first() {
        let raw = "_kpiid=abc;_kpwtkn=tok123;_kawlt=def;_kau=ghi;_kahai=jkl";
        let got = cookies_for(raw, "https://page.kakao.com");
        for name in ["_kpiid=abc", "_kpwtkn=tok123", "_kawlt=def", "_kau=ghi", "_kahai=jkl"] {
            assert!(got.contains(name), "нет {name} в {got:?}");
        }
    }

    #[test]
    fn skips_set_cookie_attributes_from_file() {
        let raw = "Domain=.kakao.com;_kpiid=abc;Max-Age=7200;_kpwtkn=tok123;Path=/;Secure;HttpOnly";
        let got = cookies_for(raw, "https://page.kakao.com");
        assert!(got.contains("_kpiid=abc"));
        assert!(got.contains("_kpwtkn=tok123"));
        for junk in ["Domain=", "Max-Age=", "Path=", "Secure", "HttpOnly"] {
            assert!(!got.contains(junk), "мусор {junk:?} попал в куки: {got:?}");
        }
    }

    #[test]
    fn cookies_are_visible_on_refresh_host() {
        let got = cookies_for("_kpiid=abc;_kpwtkn=tok123", "https://bff-page.kakao.com");
        assert!(got.contains("_kpiid=abc"), "{got:?}");
        assert!(got.contains("_kpwtkn=tok123"), "{got:?}");
    }

    #[test]
    fn round_trips_jar_dump_format() {
        // sync_to_file пишет ровно это: "name=value; name=value".
        let raw = "_kpiid=abc; _kpwtkn=tok123; _kau=ghi";
        let got = cookies_for(raw, "https://page.kakao.com");
        assert!(got.contains("_kpiid=abc"), "{got:?}");
        assert!(got.contains("_kpwtkn=tok123"), "{got:?}");
        assert!(got.contains("_kau=ghi"), "{got:?}");
    }

    #[test]
    fn skips_garbage_entries() {
        let raw = ";_kpiid=abc;;not-a-pair;_bad name=v;_kpwtkn=tok123";
        let got = cookies_for(raw, "https://page.kakao.com");
        assert!(got.contains("_kpiid=abc"), "{got:?}");
        assert!(got.contains("_kpwtkn=tok123"), "{got:?}");
        assert!(!got.contains("not-a-pair"), "{got:?}");
    }

    #[test]
    fn keeps_equals_signs_inside_values() {
        let raw = "_kpwtkn=eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0..abc;__T_SECURE=1";
        let got = cookies_for(raw, "https://page.kakao.com");
        assert!(got.contains("_kpwtkn=eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0..abc"), "{got:?}");
        assert!(got.contains("__T_SECURE=1"), "{got:?}");
    }
}
