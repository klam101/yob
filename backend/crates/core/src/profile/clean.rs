//! Fixes text-extraction quirks before the resume is parsed, so every parsing
//! rule sees plain text. One function per quirk; `clean` runs them in order.
//!
//! The quirks come from how PDFs and DOCX files store text, not from what
//! the resume says, so each one can be recognized and undone:
//!
//! 1. Compatibility characters: ligatures ("ﬁ"), full-width letters.
//! 2. Hyphen look-alikes: LaTeX and Word often emit U+2010/U+2011 instead
//!    of "-", which breaks date ranges ("Jan 2024 ‑ May 2025").
//! 3. Icon fonts (Font Awesome and Word's Symbol bullets) live in the
//!    Unicode Private Use Area and come out as junk characters.
//! 4. Small caps: LaTeX small-caps fonts map the dotted "i" back to a
//!    lowercase letter ("SOFTWARE QUALiTY ASSURANCE").
//! 5. Words hyphenated across a line break ("deliv-" / "ering").

use unicode_normalization::UnicodeNormalization;

/// Runs every fix, in order.
pub fn clean(text: &str) -> String {
    clean_with_report(text).0
}

/// A fix worth showing the user: it changed what a word says, not just how
/// it's encoded. Ligatures, hyphen look-alikes and stray icons aren't
/// reported; nothing about them needs checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub kind: FixKind,
    /// The words (or address) as they read after the fix, to find the
    /// fields that contain them.
    pub words: Vec<String>,
    /// What the file had, readable ("deliv- / ering", "QUALiTY").
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixKind {
    SmallCaps,
    JoinedWord,
    IconLink,
}

/// `clean`, plus the fixes worth showing.
pub fn clean_with_report(text: &str) -> (String, Vec<Fix>) {
    let mut fixes = Vec::new();
    let text = compatibility(text);
    let text = hyphens(&text);
    let text = icons_reported(&text, &mut fixes);
    let text = small_caps_reported(&text, &mut fixes);
    let text = line_break_hyphens_reported(&text, &mut fixes);
    (trailing_space(&text), fixes)
}

/// NFKC: ligatures and full-width forms become plain letters ("ﬁ" -> "fi").
fn compatibility(text: &str) -> String {
    text.nfkc().collect()
}

/// Hyphen look-alikes become "-"; soft hyphens and zero-width characters
/// are dropped. En and em dashes are kept: they mean something different.
fn hyphens(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' | '\u{FE63}' | '\u{FF0D}' => {
                Some('-')
            }
            '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' => None,
            c => Some(c),
        })
        .collect()
}

fn private_use(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c) || c >= '\u{F0000}'
}

/// Font Awesome code points (versions 4 to 6) that mean a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Icon {
    Github,
    Linkedin,
    /// Envelope, phone, home, globe, map pin, link: contact decorations.
    Other,
    /// Word's Symbol and Wingdings bullets.
    Bullet,
}

fn icon(c: char) -> Option<Icon> {
    match c as u32 {
        0xF09B | 0xF092 | 0xF113 => Some(Icon::Github),
        0xF08C | 0xF0E1 => Some(Icon::Linkedin),
        0xF0B7 | 0xF0A7 | 0xF0A8 | 0xF0D8 | 0xF076 | 0xF0FC | 0xF06E | 0xF0B2 | 0xF0A0 => {
            Some(Icon::Bullet)
        }
        _ if private_use(c) => Some(Icon::Other),
        _ => None,
    }
}

/// Words that label a link rather than name an account ("GitHub", "Demo").
pub const LINK_LABELS: &[&str] = &[
    "github",
    "gitlab",
    "linkedin",
    "code",
    "repo",
    "repository",
    "source",
    "link",
    "demo",
    "live",
    "website",
    "site",
    "portfolio",
    "app",
    "paper",
    "video",
];

/// Removes icon-font characters. A GitHub or LinkedIn icon in front of a
/// bare user name ("\u{F092} jdoe") becomes that account's address, which
/// is what the icon means. A Symbol-font bullet at the start of a line
/// becomes "•".
#[cfg(test)]
fn icons(text: &str) -> String {
    icons_reported(text, &mut Vec::new())
}

fn icons_reported(text: &str, fixes: &mut Vec<Fix>) -> String {
    let mut out = String::with_capacity(text.len());
    for (n, line) in text.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let mut at_start = true;
        let mut chars = line.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            let Some(kind) = icon(c) else {
                at_start &= c.is_whitespace();
                out.push(c);
                continue;
            };
            let rest = line[i + c.len_utf8()..].trim_start();
            let handle: &str = rest
                .split(|ch: char| ch.is_whitespace() || matches!(ch, '|' | ',' | ';'))
                .next()
                .unwrap_or("");
            let is_handle = !handle.is_empty()
                && handle.len() <= 39
                && handle
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
                && !LINK_LABELS.contains(&handle.to_lowercase().as_str());
            let mut link = |site: &str, prefix: &str| {
                let address = format!("{prefix}{handle}");
                fixes.push(Fix {
                    kind: FixKind::IconLink,
                    words: vec![address.clone()],
                    before: format!("{site} icon, then \"{handle}\""),
                    after: address,
                });
                out.push_str(prefix);
            };
            match kind {
                Icon::Github if is_handle => link("GitHub", "github.com/"),
                Icon::Linkedin if is_handle => link("LinkedIn", "linkedin.com/in/"),
                Icon::Bullet if at_start => out.push('•'),
                _ if at_start
                    && !rest.is_empty()
                    && kind == Icon::Other
                    && looks_like_text(rest) =>
                {
                    // An unknown icon starting a sentence is most likely a bullet.
                    out.push('•')
                }
                _ => {}
            }
            if matches!(kind, Icon::Github | Icon::Linkedin) && is_handle {
                // Skip the space between the icon and the handle.
                while chars.peek().is_some_and(|(_, ch)| ch.is_whitespace()) {
                    chars.next();
                }
            }
            at_start = false;
        }
    }
    out
}

/// A sentence rather than contact details: starts with a capital and has
/// at least four words, none of them an email or a phone number.
fn looks_like_text(s: &str) -> bool {
    s.chars().next().is_some_and(char::is_uppercase)
        && s.split_whitespace().count() >= 4
        && !s.contains('@')
        && s.chars().filter(char::is_ascii_digit).count() < 7
}

/// A word in small caps: every letter is uppercase except "i".
fn small_caps_word(w: &str) -> bool {
    let letters: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
    letters.iter().any(|c| c.is_uppercase())
        && letters.iter().all(|&c| c.is_uppercase() || c == 'i')
}

/// "QUALiTY" (an "i" between capitals) or "iNTERN" (an "i" before three or
/// more capitals), which normal writing never has. "iOS" isn't enough.
fn clearly_small_caps(w: &str) -> bool {
    let letters: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
    let inner = letters
        .windows(3)
        .any(|t| t[0].is_uppercase() && t[1] == 'i' && t[2].is_uppercase());
    let leading = letters.len() >= 4
        && letters[0] == 'i'
        && letters[1..].iter().all(|c| c.is_uppercase() || *c == 'i')
        && letters[1..4].iter().all(|c| c.is_uppercase());
    small_caps_word(w) && (inner || leading)
}

/// Upper-cases small-caps words, but only on lines with an unmistakable one
/// ("QUALiTY"), so "iOS" in a normal sentence is left alone.
#[cfg(test)]
fn small_caps(text: &str) -> String {
    small_caps_reported(text, &mut Vec::new())
}

fn small_caps_reported(text: &str, fixes: &mut Vec<Fix>) -> String {
    text.split('\n')
        .map(|line| {
            if !line.split_whitespace().any(clearly_small_caps) {
                return line.to_owned();
            }
            let mut fixed: Vec<(String, String)> = Vec::new();
            let out = line
                .split(' ')
                .map(|w| {
                    if small_caps_word(w) && w.contains('i') {
                        let up = w.to_uppercase();
                        fixed.push((w.to_owned(), up.clone()));
                        up
                    } else {
                        w.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            if !fixed.is_empty() {
                let join = |f: fn(&(String, String)) -> &str| {
                    fixed.iter().map(f).collect::<Vec<_>>().join(" ")
                };
                fixes.push(Fix {
                    kind: FixKind::SmallCaps,
                    words: fixed.iter().map(|(_, up)| up.clone()).collect(),
                    before: join(|p| p.0.as_str()),
                    after: join(|p| p.1.as_str()),
                });
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Prefixes that make a real compound word ("full-stack", "cross-team").
const COMPOUND_PREFIXES: &[&str] = &[
    "self", "full", "real", "cross", "multi", "non", "co", "well", "end", "front", "back",
    "client", "server", "user", "data", "cloud", "high", "low", "long", "short", "open", "part",
    "semi", "pre", "post", "sub", "anti", "cost", "time", "large", "small", "fast", "first",
    "third", "in", "on", "off", "re", "e", "web", "mobile", "team", "test", "event", "rule", "AI",
    "ai", "API", "api", "UI", "ui", "fine", "deep", "hands", "best", "top", "mid",
];

/// Joins a word split across two lines ("deliv-" + "ering" -> "delivering").
/// Only when the next line starts in lowercase, so "Jan 2024 -" / "Present"
/// stays apart. The whole next line is joined on, as it continues the same
/// sentence. After a compound prefix the hyphen is kept ("full-" + "stack").
#[cfg(test)]
fn line_break_hyphens(text: &str) -> String {
    line_break_hyphens_reported(text, &mut Vec::new())
}

fn line_break_hyphens_reported(text: &str, fixes: &mut Vec<Fix>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut join = false;
    for line in text.split('\n') {
        let next = line.trim_start();
        if join
            && next.chars().next().is_some_and(char::is_lowercase)
            && let Some(prev) = out.last_mut()
        {
            let word = prev[..prev.len() - 1]
                .rsplit(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap_or("");
            // A wrapped link keeps its hyphen ("linkedin.com/in/jane-" + "doe").
            let token = prev.rsplit(char::is_whitespace).next().unwrap_or("");
            let in_link = token.contains('/') || token.contains('@');
            if !COMPOUND_PREFIXES.contains(&word) && !in_link {
                let head = word.to_owned();
                let tail: String = next.chars().take_while(|c| c.is_alphanumeric()).collect();
                // The joined word with a few words after it, so the fix is
                // matched to the one bullet it came from.
                let context: String = next[tail.len()..].chars().take(30).collect();
                prev.pop();
                fixes.push(Fix {
                    kind: FixKind::JoinedWord,
                    words: vec![format!("{head}{tail}{context}").trim_end().to_owned()],
                    before: format!("{head}- / {tail}"),
                    after: format!("{head}{tail}"),
                });
            }
            prev.push_str(next);
        } else {
            out.push(line.to_owned());
        }
        let last = out.last_mut().unwrap();
        let mut tail = last.trim_end().chars().rev();
        join = tail.next() == Some('-') && tail.next().is_some_and(char::is_alphabetic);
        if join {
            last.truncate(last.trim_end().len());
        }
    }
    out.join("\n")
}

fn trailing_space(text: &str) -> String {
    text.split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligatures_and_wide_forms() {
        assert_eq!(
            compatibility("ﬁnance ｅｆｆｉｃｉｅｎｔ"),
            "finance efficient"
        );
    }

    #[test]
    fn hyphen_look_alikes() {
        assert_eq!(hyphens("Jan 2026 \u{2011} Oct 2026"), "Jan 2026 - Oct 2026");
        assert_eq!(
            hyphens("full\u{2010}stack co\u{00AD}op\u{200B}"),
            "full-stack coop"
        );
        assert_eq!(hyphens("2020 – 2022 — note"), "2020 – 2022 — note");
    }

    #[test]
    fn icon_fonts() {
        assert_eq!(
            icons("\u{F10B} (555) 555-0100 | \u{F0E0} a@example.com"),
            " (555) 555-0100 |  a@example.com"
        );
        assert_eq!(icons("\u{F092} jdoe-example"), "github.com/jdoe-example");
        assert_eq!(icons("\u{F08C} jane-doe |"), "linkedin.com/in/jane-doe |");
        // A link label isn't an account name.
        assert_eq!(icons("Sunset \u{F092} GitHub"), "Sunset  GitHub");
        assert_eq!(icons("\u{F0B7} Built a thing"), "• Built a thing");
        assert_eq!(
            icons("  \u{E001} Led a team of four people"),
            "  • Led a team of four people"
        );
    }

    #[test]
    fn small_caps_lines() {
        assert_eq!(
            small_caps("SOFTWARE QUALiTY ASSURANCE TESTER Jan 2026 - Oct 2026"),
            "SOFTWARE QUALITY ASSURANCE TESTER Jan 2026 - Oct 2026"
        );
        assert_eq!(
            small_caps("BACHELOR OF SCiENCE iN COMPUTER SCiENCE"),
            "BACHELOR OF SCIENCE IN COMPUTER SCIENCE"
        );
        assert_eq!(
            small_caps("DATA ANALYST iNTERN May 2024 - Aug 2024"),
            "DATA ANALYST INTERN May 2024 - Aug 2024"
        );
        // Normal sentences keep "iOS".
        assert_eq!(small_caps("Shipped the iOS app"), "Shipped the iOS app");
        assert_eq!(
            small_caps("QA for iOS and ANDROiD"),
            "QA for IOS and ANDROID"
        );
    }

    #[test]
    fn hyphenated_line_breaks() {
        assert_eq!(
            line_break_hyphens("Cut time to deliv-\nering features"),
            "Cut time to delivering features"
        );
        assert_eq!(
            line_break_hyphens("Built a full-\nstack app"),
            "Built a full-stack app"
        );
        assert_eq!(
            line_break_hyphens("Jan 2024 -\nPresent"),
            "Jan 2024 -\nPresent"
        );
        assert_eq!(
            line_break_hyphens("linkedin.com/in/jane-\ndoe-example"),
            "linkedin.com/in/jane-doe-example"
        );
        assert_eq!(
            line_break_hyphens("Acme -\nAustin, TX"),
            "Acme -\nAustin, TX"
        );
    }

    #[test]
    fn whole_pass() {
        let raw = "\u{F092} kl-example\nSOFTWARE QUALiTY TESTER Jan 2026 \u{2011} Oct 2026  \n\u{F0B7} Improved deliv\u{2011}\nering by 20%";
        assert_eq!(
            clean(raw),
            "github.com/kl-example\nSOFTWARE QUALITY TESTER Jan 2026 - Oct 2026\n• Improved delivering by 20%"
        );
    }
}
