//! Chinese punctuation translation (core-session.md §5.6, overlays.md §7.1, `contracts/punctuation/policy.json`): a fixed map, alternating quotes, and nested book-title marks, with a second set of characters for the quote and square-bracket keys (`styles`) that the host picks from preferences. Quotes always alternate and book titles always nest, whether pairing is on or not; pairing only decides whether a host-side auto-close pays back the nesting count.

/// `policy.json` contract version 1, `simple` in file order.
const SIMPLE: [(u8, &str); 15] = [
    (b',', "，"),
    (b'.', "。"),
    (b'?', "？"),
    (b'!', "！"),
    (b';', "；"),
    (b':', "："),
    (b'(', "（"),
    (b')', "）"),
    (b'[', "【"),
    (b']', "】"),
    (b'\\', "、"),
    (b'`', "·"),
    (b'$', "￥"),
    (b'^', "……"),
    (b'_', "——"),
];

const DOUBLE_QUOTE: (&str, &str) = ("“", "”");
const SINGLE_QUOTE: (&str, &str) = ("‘", "’");

/// `policy.json` 的 `styles.cornerQuotes`：直角引号下 `"` 和 `'` 的两组字。第 3 版起 `{` `}` 也出这两组字，与样式无关——弯引号样式下「」『』因此和弯引号并存。
const CORNER_DOUBLE_QUOTE: (&str, &str) = ("『", "』");
const CORNER_SINGLE_QUOTE: (&str, &str) = ("「", "」");

/// `policy.json` 的 `styles.hollowBrackets`：空心方头括号替换 `[` `]` 在 `SIMPLE` 里的字。
const HOLLOW_BRACKETS: [(u8, &str); 2] = [(b'[', "〖"), (b']', "〗")];

/// `policy.json` 第 4 版的 `enumerationComma`：可被偏好关掉的顿号键。`\` 同时在 `SIMPLE` 里（第 1 版起的无条件映射，`translate` 按开关拦它）；`/` 只在这一段，开关没打开时它是普通字符。
const ENUMERATION_COMMA: [(u8, &str); 2] = [(b'\\', "、"), (b'/', "、")];

/// 引号键出哪一组字。`Corner` 下 `'` 出「」、`"` 出『』，按键对应，不按语义对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuoteMarks {
    #[default]
    Curly,
    Corner,
}

/// `[` `]` 出实心【】还是空心〖〗。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BracketMarks {
    #[default]
    Solid,
    Hollow,
}

/// 整套标点键位的档案。`Standard` 是契约里的那张表；`MacosPinyin` 对齐 macOS 自带「简体拼音」的键位，在它之上多出两件事：`~` 出 `～`、`|` 出 `｜`，并且 `{` `}` 按嵌套深度出「『』」，见 `PunctuationPolicy::translate_in_context`。
///
/// 档案只改键位，不改引号样式：`MacosPinyin` 下 `'` `"` 仍按 `quotes` 出弯引号或直角引号。契约（`policy.json`）暂未收录这个档案，第一个宿主接入它时再升版本，在此之前宿主的手抄表不需要动。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PunctuationProfile {
    #[default]
    Standard,
    MacosPinyin,
}

/// 已经解析好的标点样式与顿号键：宿主从偏好算出它（`corner_when_traditional` 在那之前就按繁体开关定成两者之一，「至少留一个」也已归一），引擎只认确定的值。缺省就是契约第 1 版的表加「只有 `\` 出顿号」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PunctuationMarks {
    pub quotes: QuoteMarks,
    pub brackets: BracketMarks,
    /// `\` 键出顿号「、」。
    pub enumeration_comma_backslash: bool,
    /// `/` 键出顿号「、」。
    pub enumeration_comma_slash: bool,
    pub profile: PunctuationProfile,
}

impl Default for PunctuationMarks {
    fn default() -> Self {
        Self {
            quotes: QuoteMarks::Curly,
            brackets: BracketMarks::Solid,
            enumeration_comma_backslash: true,
            enumeration_comma_slash: false,
            profile: PunctuationProfile::Standard,
        }
    }
}

/// `MacosPinyin` 档案里 `simple` 之外多出来的单键映射。`$` 不在这里：macOS 的 `¥`（U+00A5）就是 host-api 在半角下对 `$` 标点尾巴做的改写（见 `2026-10-15-currency-punctuation-character-width.md`），引擎照旧出 `￥`，全角时宿主仍得到 `￥`。
const MACOS_PINYIN_SIMPLE: [(u8, &str); 2] = [(b'~', "～"), (b'|', "｜")];

const CORNER_NESTING_OPENING_INPUT: u8 = b'{';
const CORNER_NESTING_CLOSING_INPUT: u8 = b'}';
/// 直角引号的嵌套层：外层「」，内层『』，再往里交替。
const CORNER_NESTING_OUTER: (&str, &str) = ("「", "」");
const CORNER_NESTING_INNER: (&str, &str) = ("『", "』");

/// 光标前文字里还没合上的直角引号个数，以及最里面那一个是「还是『。只看「『」『，不碰别的括号；`」` `』` 不论配不配得上都弹出一层，用户手打或粘贴出的不成对文字不会让深度一直涨上去。
fn open_corner_quotes(preceding: &str) -> (u32, Option<char>) {
    let mut stack: Vec<char> = Vec::new();
    for ch in preceding.chars() {
        match ch {
            '「' | '『' => stack.push(ch),
            '」' | '』' => {
                stack.pop();
            }
            _ => {}
        }
    }
    (
        u32::try_from(stack.len()).unwrap_or(u32::MAX),
        stack.last().copied(),
    )
}

const NESTED_OPENING_INPUT: u8 = b'<';
const NESTED_CLOSING_INPUT: u8 = b'>';
const NESTED_OPENING: &str = "《";
const NESTED_OPENING_INNER: &str = "〈";
const NESTED_CLOSING: &str = "》";
const NESTED_CLOSING_INNER: &str = "〉";

#[derive(Debug, Clone)]
pub struct PunctuationPolicy {
    paired_enabled: bool,
    marks: PunctuationMarks,
    next_double_quote_is_opening: bool,
    next_single_quote_is_opening: bool,
    next_corner_single_is_opening: bool,
    next_corner_double_is_opening: bool,
    book_title_nesting: u32,
    /// `MacosPinyin` 档案下 `{` `}` 的嵌套深度。宿主读不到光标前文字时只有它可用；读得到时每次都以文字为准并回写这里，之后某次读不到也接得上。
    corner_nesting: u32,
}

impl Default for PunctuationPolicy {
    fn default() -> Self {
        Self {
            paired_enabled: true,
            marks: PunctuationMarks::default(),
            next_double_quote_is_opening: true,
            next_single_quote_is_opening: true,
            next_corner_single_is_opening: true,
            next_corner_double_is_opening: true,
            book_title_nesting: 0,
            corner_nesting: 0,
        }
    }
}

impl PunctuationPolicy {
    /// 将 ASCII 按键转成中文标点并推进引号与嵌套状态；没有映射的键（如 `@`、`-`）返回 `None`。读不到光标前文字，等价于 `translate_in_context(key, None)`。
    pub fn translate(&mut self, key: u8) -> Option<&'static str> {
        self.translate_in_context(key, None)
    }

    /// 同 `translate`，另带宿主读到的光标前文字（`None` 表示读不到，`Some("")` 表示读到了而且前面什么都没有）。
    ///
    /// 只有 `MacosPinyin` 档案下的 `{` `}` 用它：
    /// - `{`：光标前已有几层没合上的「『，就出第几层。偶数层（含 0）出「，奇数层出『。
    /// - `}`：合上最里面那一层，是「就出」，是『就出』；一层都没有时出」，且不让深度变负。
    ///
    /// 读不到文字时退回内部的 `corner_nesting` 计数，奇偶规则相同。它回退不了：用户删掉括号后计数仍然偏高，所以宿主能读文字就应该传。macOS 自带输入法在读不到文字的应用里每次都出「，这里不照搬。
    pub fn translate_in_context(
        &mut self,
        key: u8,
        preceding: Option<&str>,
    ) -> Option<&'static str> {
        if self.marks.profile == PunctuationProfile::MacosPinyin {
            if let Some(output) = self.translate_macos_pinyin(key, preceding) {
                return Some(output);
            }
        }
        if self.marks.brackets == BracketMarks::Hollow {
            if let Some(&(_, output)) = HOLLOW_BRACKETS.iter().find(|(input, _)| *input == key) {
                return Some(output);
            }
        }
        // 顿号键可被偏好关掉（enumerationComma 段）：没启用的键返回 `None`，按普通字符交回宿主。`\` 同时在 `SIMPLE` 里，由这段先拦下；`/` 不在 `SIMPLE`，启用了才出顿号。
        if let Some(&(input, output)) = ENUMERATION_COMMA.iter().find(|(input, _)| *input == key) {
            let enabled = match input {
                b'\\' => self.marks.enumeration_comma_backslash,
                _ => self.marks.enumeration_comma_slash,
            };
            return enabled.then_some(output);
        }
        if let Some(&(_, output)) = SIMPLE.iter().find(|(input, _)| *input == key) {
            return Some(output);
        }
        let corner = self.marks.quotes == QuoteMarks::Corner;
        // Without host pairing this alternation is the only way to type a closing quote, and a host that does pair rewrites the closing quote itself, so it runs either way (punctuation_policy.cpp:12).
        // 交替状态按键保存，和出哪一组字无关：换样式不重置它。
        match key {
            b'"' => Some(alternate(
                &mut self.next_double_quote_is_opening,
                if corner {
                    CORNER_DOUBLE_QUOTE
                } else {
                    DOUBLE_QUOTE
                },
            )),
            b'\'' => Some(alternate(
                &mut self.next_single_quote_is_opening,
                if corner {
                    CORNER_SINGLE_QUOTE
                } else {
                    SINGLE_QUOTE
                },
            )),
            // `{` `}` 的直角引号不吃样式：它们就是 cornerQuotes 里同一组字，交替状态与 `'` `"` 分开保存。
            b'{' => Some(alternate(
                &mut self.next_corner_single_is_opening,
                CORNER_SINGLE_QUOTE,
            )),
            b'}' => Some(alternate(
                &mut self.next_corner_double_is_opening,
                CORNER_DOUBLE_QUOTE,
            )),
            NESTED_OPENING_INPUT => {
                self.book_title_nesting += 1;
                Some(if self.book_title_nesting == 1 {
                    NESTED_OPENING
                } else {
                    NESTED_OPENING_INNER
                })
            }
            // An unmatched `>` leaves the count at zero, so the next `<` still opens the outer pair.
            NESTED_CLOSING_INPUT => {
                if self.book_title_nesting == 0 {
                    return Some(NESTED_CLOSING);
                }
                self.book_title_nesting -= 1;
                Some(if self.book_title_nesting == 0 {
                    NESTED_CLOSING
                } else {
                    NESTED_CLOSING_INNER
                })
            }
            _ => None,
        }
    }

    /// `MacosPinyin` 档案独有的键位；其余键返回 `None`，交回通用的转换。
    fn translate_macos_pinyin(&mut self, key: u8, preceding: Option<&str>) -> Option<&'static str> {
        if let Some(&(_, output)) = MACOS_PINYIN_SIMPLE.iter().find(|(input, _)| *input == key) {
            return Some(output);
        }
        let (depth, innermost) = match preceding {
            Some(text) => open_corner_quotes(text),
            None => (
                self.corner_nesting,
                // 没有文字可看时，奇数层的最里面一层是「，偶数层是『。
                match self.corner_nesting {
                    0 => None,
                    depth if depth % 2 == 1 => Some('「'),
                    _ => Some('『'),
                },
            ),
        };
        match key {
            CORNER_NESTING_OPENING_INPUT => {
                self.corner_nesting = depth.saturating_add(1);
                let (opening, _) = if depth % 2 == 0 {
                    CORNER_NESTING_OUTER
                } else {
                    CORNER_NESTING_INNER
                };
                Some(opening)
            }
            CORNER_NESTING_CLOSING_INPUT => {
                self.corner_nesting = depth.saturating_sub(1);
                let (_, closing) = if innermost == Some('『') {
                    CORNER_NESTING_INNER
                } else {
                    CORNER_NESTING_OUTER
                };
                Some(closing)
            }
            _ => None,
        }
    }

    pub fn set_paired_enabled(&mut self, enabled: bool) {
        self.paired_enabled = enabled;
    }

    pub fn set_marks(&mut self, marks: PunctuationMarks) {
        self.marks = marks;
    }

    /// The host emitted the closing half of `opening` itself; only `<` with pairing on and a positive nesting count changes anything.
    pub fn balance_after_auto_close(&mut self, opening: u8) {
        if self.paired_enabled && opening == NESTED_OPENING_INPUT && self.book_title_nesting > 0 {
            self.book_title_nesting -= 1;
        }
    }
}

fn alternate(
    next_is_opening: &mut bool,
    (opening, closing): (&'static str, &'static str),
) -> &'static str {
    let output = if *next_is_opening { opening } else { closing };
    *next_is_opening = !*next_is_opening;
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translate_all(policy: &mut PunctuationPolicy, keys: &str) -> Vec<Option<&'static str>> {
        keys.bytes().map(|key| policy.translate(key)).collect()
    }

    fn macos_pinyin() -> PunctuationPolicy {
        let mut policy = PunctuationPolicy::default();
        policy.set_marks(PunctuationMarks {
            profile: PunctuationProfile::MacosPinyin,
            ..PunctuationMarks::default()
        });
        policy
    }

    fn joined(outputs: Vec<Option<&'static str>>) -> String {
        outputs
            .into_iter()
            .map(|output| output.unwrap_or("?"))
            .collect()
    }

    /// 缺省档案不认 `~` `|`，`{` `}` 也还是各自交替，和加档案之前逐字相同。
    #[test]
    fn standard_profile_is_unchanged() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(translate_all(&mut policy, "~|"), [None, None]);
        assert_eq!(policy.translate(b'$'), Some("￥"));
        assert_eq!(joined(translate_all(&mut policy, "{{}}")), "「」『』");
    }

    /// 单键映射：`~` `|` 多出来；`$` 仍是引擎的 `￥`，半角下改成 `¥` 是 host-api 的事。
    #[test]
    fn macos_pinyin_adds_two_keys() {
        let mut policy = macos_pinyin();
        assert_eq!(joined(translate_all(&mut policy, "~|")), "～｜");
        assert_eq!(policy.translate(b'$'), Some("\u{ffe5}"));
        // 其余键仍是契约里的表。
        assert_eq!(
            joined(translate_all(&mut policy, ",.?!;:()[]\\`^_")),
            "，。？！；：（）【】、·……——"
        );
        assert_eq!(translate_all(&mut policy, "@#%&*+-=/"), vec![None; 9]);
    }

    /// 以下序列都是在 macOS 简体拼音里实际打出来的结果。
    #[test]
    fn macos_pinyin_nests_corner_brackets() {
        for (keys, expected) in [
            ("{{}}", "「『』」"),
            ("{}{}", "「」「」"),
            ("{{{{}}}}", "「『「『』」』」"),
            ("}", "」"),
            ("}{}{", "」「」「"),
        ] {
            let mut policy = macos_pinyin();
            assert_eq!(joined(translate_all(&mut policy, keys)), expected, "{keys}");
        }
    }

    /// 比 macOS 做得更好的一处：多出来的 `}` 一律出」，不会像 macOS 那样 `』」』」` 来回翻。深度也不会变负。
    #[test]
    fn stray_closing_bracket_is_always_closing() {
        let mut policy = macos_pinyin();
        assert_eq!(joined(translate_all(&mut policy, "}}}}")), "」」」」");
        assert_eq!(joined(translate_all(&mut policy, "{}}{")), "「」」「");
    }

    #[test]
    fn context_decides_the_level_when_the_host_can_read_it() {
        let mut policy = macos_pinyin();
        for (key, preceding, expected) in [
            (b'{', "", "「"),
            // 中间隔着正文，只有读文字才知道还在「」里面。
            (b'{', "他说「你好", "『"),
            (b'{', "他说「你好『再见", "「"),
            (b'}', "他说「你好『再见", "』"),
            (b'}', "他说「你好", "」"),
            (b'}', "他说「你好『再见』然后", "」"),
            (b'}', "他说「你好」", "」"),
            (b'{', "他说「你好」", "「"),
            // 手打的「（比如直角引号样式下 `'` 出的）也算一层。
            (b'{', "「", "『"),
            // 不配对的收尾字符不会让深度变负。
            (b'{', "』」」", "「"),
        ] {
            assert_eq!(
                policy.translate_in_context(key, Some(preceding)),
                Some(expected),
                "{} after {preceding:?}",
                key as char
            );
        }
    }

    /// 读到文字时以文字为准，之后读不到也接得上；用户删掉括号后再读到文字，计数跟着文字走。
    #[test]
    fn context_resyncs_the_fallback_counter() {
        let mut policy = macos_pinyin();
        assert_eq!(
            policy.translate_in_context(b'{', Some("他说「你好")),
            Some("『")
        );
        // 这一次读不到：计数是 2，所以 `}` 合上的是『。
        assert_eq!(policy.translate(b'}'), Some("』"));
        assert_eq!(policy.translate(b'}'), Some("」"));
        // 计数此刻是 0。再按一次 `{` 让它涨到 2，然后用户删光了括号：读到的文字说深度是 0，计数跟着文字回到 1，而不是接着涨到 3。
        assert_eq!(
            policy.translate_in_context(b'{', Some("他说「你好")),
            Some("『")
        );
        assert_eq!(policy.translate_in_context(b'{', Some("")), Some("「"));
        assert_eq!(policy.translate(b'}'), Some("」"));
        assert_eq!(policy.translate(b'}'), Some("」"));
    }

    /// 档案只管 `{` `}` `~` `|` `$`：书名号、引号交替和顿号键不受影响。
    #[test]
    fn macos_pinyin_leaves_the_other_state_machines_alone() {
        let mut policy = macos_pinyin();
        assert_eq!(
            joined(translate_all(&mut policy, "<<>>\"\"''")),
            "《〈〉》“”‘’"
        );
        assert_eq!(policy.translate(b'/'), None);
    }

    /// The table is the one `shared/contracts/punctuation/policy.json` defines, and `policy.h`, which English-mode output on macOS, IBus and Fcitx5 reads, carries the same rows, so a key gives the same mark in either mode.
    #[test]
    fn matches_the_shared_contract() {
        let spec: serde_json::Value = serde_json::from_str(include_str!(
            "../../../shared/contracts/punctuation/policy.json"
        ))
        .expect("policy.json");
        assert_eq!(spec["contractVersion"], 4);
        let simple = spec["simple"].as_array().expect("simple");
        assert_eq!(simple.len(), SIMPLE.len());
        for ((input, output), row) in SIMPLE.iter().zip(simple) {
            assert_eq!(
                row["input"].as_str().map(str::as_bytes),
                Some(&[*input][..])
            );
            assert_eq!(row["output"], *output);
        }
        let alternating = spec["alternating"].as_array().expect("alternating");
        assert_eq!(alternating.len(), 4);
        for (row, (input, (opening, closing))) in alternating.iter().zip([
            ("\"", DOUBLE_QUOTE),
            ("'", SINGLE_QUOTE),
            ("{", CORNER_SINGLE_QUOTE),
            ("}", CORNER_DOUBLE_QUOTE),
        ]) {
            assert_eq!(row["input"], input);
            assert_eq!(row["opening"], opening);
            assert_eq!(row["closing"], closing);
        }
        let nested = &spec["nested"];
        assert_eq!(
            nested["openingInput"].as_str().map(str::as_bytes),
            Some(&[NESTED_OPENING_INPUT][..])
        );
        assert_eq!(
            nested["closingInput"].as_str().map(str::as_bytes),
            Some(&[NESTED_CLOSING_INPUT][..])
        );
        assert_eq!(nested["opening"], NESTED_OPENING);
        assert_eq!(nested["nestedOpening"], NESTED_OPENING_INNER);
        assert_eq!(nested["closing"], NESTED_CLOSING);
        assert_eq!(nested["nestedClosing"], NESTED_CLOSING_INNER);

        let styles = &spec["styles"];
        let corner = styles["cornerQuotes"].as_array().expect("cornerQuotes");
        assert_eq!(corner.len(), 4);
        for (row, (input, (opening, closing))) in corner.iter().zip([
            ("\"", CORNER_DOUBLE_QUOTE),
            ("'", CORNER_SINGLE_QUOTE),
            ("{", CORNER_SINGLE_QUOTE),
            ("}", CORNER_DOUBLE_QUOTE),
        ]) {
            assert_eq!(row["input"], input);
            assert_eq!(row["opening"], opening);
            assert_eq!(row["closing"], closing);
        }
        let hollow = styles["hollowBrackets"].as_array().expect("hollowBrackets");
        assert_eq!(hollow.len(), HOLLOW_BRACKETS.len());
        for ((input, output), row) in HOLLOW_BRACKETS.iter().zip(hollow) {
            assert_eq!(
                row["input"].as_str().map(str::as_bytes),
                Some(&[*input][..])
            );
            assert_eq!(row["output"], *output);
        }
        let enumeration = spec["enumerationComma"]
            .as_array()
            .expect("enumerationComma");
        assert_eq!(enumeration.len(), ENUMERATION_COMMA.len());
        for ((input, output), row) in ENUMERATION_COMMA.iter().zip(enumeration) {
            assert_eq!(
                row["input"].as_str().map(str::as_bytes),
                Some(&[*input][..])
            );
            assert_eq!(row["output"], *output);
        }

        let header = include_str!("../../../shared/contracts/punctuation/policy.h");
        let literal = |input: u8| match input {
            b'\\' => "'\\\\'".to_owned(),
            b'\'' => "'\\''".to_owned(),
            _ => format!("'{}'", input as char),
        };
        assert!(header.contains(&format!("std::array<Mapping, {}> simple", SIMPLE.len())));
        for (input, output) in SIMPLE {
            let row = format!("{{{}, \"{output}\"}}", literal(input));
            assert!(header.contains(&row), "policy.h lacks {row}");
        }
        for (input, (opening, closing)) in [
            (b'"', DOUBLE_QUOTE),
            (b'\'', SINGLE_QUOTE),
            (b'{', CORNER_SINGLE_QUOTE),
            (b'}', CORNER_DOUBLE_QUOTE),
            (b'"', CORNER_DOUBLE_QUOTE),
            (b'\'', CORNER_SINGLE_QUOTE),
            (b'{', CORNER_SINGLE_QUOTE),
            (b'}', CORNER_DOUBLE_QUOTE),
        ] {
            let row = format!("{{{}, \"{opening}\", \"{closing}\"}}", literal(input));
            assert!(header.contains(&row), "policy.h lacks {row}");
        }
        assert!(header.contains("std::array<AlternatingMapping, 4> alternating"));
        assert!(header.contains("std::array<AlternatingMapping, 4> corner_quotes"));
        assert!(header.contains(&format!(
            "std::array<Mapping, {}> hollow_brackets",
            HOLLOW_BRACKETS.len()
        )));
        for (input, output) in HOLLOW_BRACKETS {
            let row = format!("{{{}, \"{output}\"}}", literal(input));
            assert!(header.contains(&row), "policy.h lacks {row}");
        }
        assert!(header.contains(&format!(
            "std::array<Mapping, {}> enumeration_comma",
            ENUMERATION_COMMA.len()
        )));
        for (input, output) in ENUMERATION_COMMA {
            let row = format!("{{{}, \"{output}\"}}", literal(input));
            assert!(header.contains(&row), "policy.h lacks {row}");
        }
        for (name, value) in [
            ("nested_opening_input", literal(NESTED_OPENING_INPUT)),
            ("nested_closing_input", literal(NESTED_CLOSING_INPUT)),
            ("nested_opening[]", format!("\"{NESTED_OPENING}\"")),
            (
                "nested_opening_inner[]",
                format!("\"{NESTED_OPENING_INNER}\""),
            ),
            ("nested_closing[]", format!("\"{NESTED_CLOSING}\"")),
            (
                "nested_closing_inner[]",
                format!("\"{NESTED_CLOSING_INNER}\""),
            ),
        ] {
            let line = format!("{name} = {value};");
            assert!(header.contains(&line), "policy.h lacks {line}");
        }
    }

    /// contracts/tests/punctuation_contract.cpp.
    #[test]
    fn contract_table() {
        assert_eq!(SIMPLE.len(), 15);
        assert_eq!(ENUMERATION_COMMA.len(), 2);
        let mut policy = PunctuationPolicy::default();
        assert_eq!(policy.translate(b'$'), Some("￥"));
        assert_eq!(policy.translate(b'_'), Some("——"));
        assert_eq!(policy.translate(b'"'), Some("“"));
        assert_eq!(policy.translate(b'"'), Some("”"));
        assert_eq!(policy.translate(b'<'), Some("《"));
        assert_eq!(policy.translate(b'<'), Some("〈"));
        for key in *b"a@-1 /" {
            assert_eq!(policy.translate(key), None);
        }
    }

    /// 契约第 4 版：`enumerationComma` 的键由各自的偏好开关决定，没打开的键返回 `None`（宿主按普通字符输出）。缺省是「只有 `\`」，`/` 因此仍是命令模式触发键。
    #[test]
    fn enumeration_comma_keys_follow_their_switches() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(policy.translate(b'\\'), Some("、"));
        assert_eq!(policy.translate(b'/'), None);

        policy.set_marks(PunctuationMarks {
            enumeration_comma_slash: true,
            ..PunctuationMarks::default()
        });
        assert_eq!(policy.translate(b'/'), Some("、"));

        policy.set_marks(PunctuationMarks {
            enumeration_comma_backslash: false,
            enumeration_comma_slash: true,
            ..PunctuationMarks::default()
        });
        assert_eq!(policy.translate(b'\\'), None);
        assert_eq!(policy.translate(b'/'), Some("、"));

        // 两个都关时谁出顿号是偏好解析处的归一职责；引擎只照开关执行，两个都关就都按普通字符。
        policy.set_marks(PunctuationMarks {
            enumeration_comma_backslash: false,
            enumeration_comma_slash: false,
            ..PunctuationMarks::default()
        });
        assert_eq!(policy.translate(b'\\'), None);
        assert_eq!(policy.translate(b'/'), None);
        assert_eq!(policy.translate(b','), Some("，"));
    }

    /// The idle map of test_input_session.cpp:850-885, as recorded in the qp_idle_chinese_punctuation golden.
    #[test]
    fn idle_map_in_recorded_order() {
        let mut policy = PunctuationPolicy::default();
        let expected = [
            "。", "“", "”", "‘", "’", "（", "）", "【", "】", "·", "￥", "……", "——", "《", "〈",
            "〉", "》", "、",
        ];
        let actual = translate_all(&mut policy, ".\"\"''()[]`$^_<<>>\\");
        assert_eq!(actual, expected.map(Some));
        let rest = translate_all(&mut policy, ",?!;:");
        assert_eq!(rest, ["，", "？", "！", "；", "："].map(Some));
    }

    #[test]
    fn quotes_toggle_independently() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(
            translate_all(&mut policy, "\"'\"'"),
            ["“", "‘", "”", "’"].map(Some)
        );
    }

    /// qp_paired_punctuation_disabled: alternation and nesting run with pairing off, and toggling pairing resets neither.
    #[test]
    fn pairing_off_still_alternates_and_nests() {
        let mut policy = PunctuationPolicy::default();
        policy.set_paired_enabled(false);
        assert_eq!(
            translate_all(&mut policy, "\"\"''<<>>"),
            ["“", "”", "‘", "’", "《", "〈", "〉", "》"].map(Some)
        );
        policy.translate(b'"');
        policy.set_paired_enabled(true);
        assert_eq!(policy.translate(b'"'), Some("”"));
    }

    #[test]
    fn unmatched_closing_never_goes_negative() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(
            translate_all(&mut policy, ">><"),
            ["》", "》", "《"].map(Some)
        );
        assert_eq!(policy.book_title_nesting, 1);
    }

    #[test]
    fn auto_close_balances_only_with_pairing() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(policy.translate(b'<'), Some("《"));
        policy.balance_after_auto_close(b'<');
        assert_eq!(policy.book_title_nesting, 0);
        assert_eq!(policy.translate(b'<'), Some("《"));

        policy.balance_after_auto_close(b'(');
        assert_eq!(policy.book_title_nesting, 1);

        policy.set_paired_enabled(false);
        policy.balance_after_auto_close(b'<');
        assert_eq!(policy.book_title_nesting, 1);
        assert_eq!(policy.translate(b'<'), Some("〈"));

        policy.set_paired_enabled(true);
        let mut empty = PunctuationPolicy::default();
        empty.balance_after_auto_close(b'<');
        assert_eq!(empty.book_title_nesting, 0);
    }

    fn styled(quotes: QuoteMarks, brackets: BracketMarks) -> PunctuationPolicy {
        let mut policy = PunctuationPolicy::default();
        policy.set_marks(PunctuationMarks {
            quotes,
            brackets,
            ..PunctuationMarks::default()
        });
        policy
    }

    #[test]
    fn corner_quotes_alternate_per_key() {
        let mut policy = styled(QuoteMarks::Corner, BracketMarks::Solid);
        assert_eq!(
            translate_all(&mut policy, "'\"'\""),
            ["「", "『", "」", "』"].map(Some)
        );
        assert_eq!(translate_all(&mut policy, "[]"), ["【", "】"].map(Some));
    }

    #[test]
    fn hollow_brackets_replace_only_square_keys() {
        let mut hollow = styled(QuoteMarks::Curly, BracketMarks::Hollow);
        let mut solid = PunctuationPolicy::default();
        assert_eq!(translate_all(&mut hollow, "[]"), ["〖", "〗"].map(Some));
        for (key, _) in SIMPLE.iter().filter(|(key, _)| !matches!(key, b'[' | b']')) {
            assert_eq!(hollow.translate(*key), solid.translate(*key));
        }
        assert_eq!(
            translate_all(&mut hollow, "\"'<>"),
            ["“", "‘", "《", "》"].map(Some)
        );
    }

    /// 契约第 3 版：`Shift+[` `Shift+]`（`{` `}`）出直角引号「」和『』，与 `'` `"` 的交替互不共享状态，也不随引号样式改变。
    #[test]
    fn shifted_brackets_give_corner_quotes() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(
            translate_all(&mut policy, "{}{{}"),
            ["「", "『", "」", "「", "』"].map(Some)
        );
        // `{` `}` 的交替与 `'` `"` 各自独立：两个弯引号键从没按过，仍出前半个。
        assert_eq!(
            translate_all(&mut policy, "\"'}{"),
            ["“", "‘", "『", "」"].map(Some)
        );
        // 直角引号样式下 `{` `}` 出的还是这两组字：样式只换 `'` `"` 的字。
        let mut corner = styled(QuoteMarks::Corner, BracketMarks::Solid);
        assert_eq!(translate_all(&mut corner, "{}"), ["「", "『"].map(Some));
        // 成对补全关着也一样交替。
        policy.set_paired_enabled(false);
        assert_eq!(translate_all(&mut policy, "{"), ["「"].map(Some));
    }

    /// 缺省样式就是契约第 1 版那张表，逐键相同。
    #[test]
    fn default_marks_match_the_contract() {
        assert_eq!(
            PunctuationMarks::default(),
            PunctuationMarks {
                quotes: QuoteMarks::Curly,
                brackets: BracketMarks::Solid,
                enumeration_comma_backslash: true,
                enumeration_comma_slash: false,
                profile: PunctuationProfile::Standard,
            }
        );
        let mut styled = styled(QuoteMarks::Curly, BracketMarks::Solid);
        let mut default = PunctuationPolicy::default();
        let keys = ",.?!;:()[]\\`$^_\"\"''<<>>";
        assert_eq!(
            translate_all(&mut styled, keys),
            translate_all(&mut default, keys)
        );
    }

    #[test]
    fn marks_do_not_reset_alternation() {
        let mut policy = PunctuationPolicy::default();
        assert_eq!(policy.translate(b'\''), Some("‘"));
        policy.set_marks(PunctuationMarks {
            quotes: QuoteMarks::Corner,
            brackets: BracketMarks::Solid,
            ..PunctuationMarks::default()
        });
        assert_eq!(policy.translate(b'\''), Some("」"));
        assert_eq!(policy.translate(b'\''), Some("「"));
    }

    #[test]
    fn pairing_off_still_alternates_corner_quotes() {
        let mut policy = styled(QuoteMarks::Corner, BracketMarks::Hollow);
        policy.set_paired_enabled(false);
        assert_eq!(
            translate_all(&mut policy, "''\"\"[]"),
            ["「", "」", "『", "』", "〖", "〗"].map(Some)
        );
    }

    #[test]
    fn balance_after_auto_close_ignores_styles() {
        let mut policy = styled(QuoteMarks::Corner, BracketMarks::Hollow);
        assert_eq!(policy.translate(b'<'), Some("《"));
        policy.balance_after_auto_close(b'[');
        policy.balance_after_auto_close(b'\'');
        assert_eq!(policy.book_title_nesting, 1);
        policy.balance_after_auto_close(b'<');
        assert_eq!(policy.book_title_nesting, 0);
        assert_eq!(policy.translate(b'\''), Some("「"));
    }
}
