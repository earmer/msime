# Agent Note: macOS 简体拼音标点档案与直角引号嵌套

Status: proposed

## Problem

维护者希望标点有一个「对齐 macOS 简体拼音」的选项。现有的 `quote_style` 与 `bracket_style` 只能换一组字，表达不了三件事：

- `~` 出 `～`、`|` 出 `｜`。引擎里 `~` `|` 没有映射。`$` 在 macOS 上出 `¥`（U+00A5），这一条不用做：host-api 在半角下本来就把 `$` 的标点尾巴改成 U+00A5（见 [人民币标点跟随全半角状态](../../implemented/bug-fix/2026-10-15-currency-punctuation-character-width.md)），全角时仍是 `￥`。
- `{` `}` 在 macOS 上按嵌套深度出「『』」：连按 `{{}}` 得到 `「『』」`，`{}{}` 得到 `「」「」`。引擎现在是两个互相独立的交替：`{` 出「」、`}` 出『』，同样的按键序列得到 `「』『」`。
- macOS 的做法依赖文档：它读光标前的文字，读不到的应用里 `{` 每次都出「，于是 `{{{{{` 得到 `「「「「「`。

macOS 上的行为是在 macOS 上逐键实测得到的，实测序列写进了引擎测试。有两点实测之外只能推断：多出来的 `}` 在 macOS 上会 `』` `」` 来回翻，看起来取决于光标前一个字符；从样本推不出它回看多远。

## Proposal

**引擎先行。** `crates/engine/src/punctuation.rs` 新增 `PunctuationProfile`（`Standard`、`MacosPinyin`），放进 `PunctuationMarks.profile`，缺省 `Standard`，缺省行为逐字不变。`MacosPinyin` 下：

- `~` `|` 优先于 `SIMPLE` 命中；`$` 不动。
- `{` 按当前深度出字：偶数层（含 0）出「，奇数层出『。
- `}` 合上最里面一层，是「出」，是『出』；一层都没有出」，且深度不为负。
- `'` `"` 不动，仍按 `quotes` 出弯引号或直角引号；书名号、顿号键也不动。

**深度的来源是文档，不是计数。** `PunctuationPolicy::translate_in_context(key, preceding)` 接收宿主读到的光标前文字，数其中没合上的「『；读不到（`None`）时退回内部计数 `corner_nesting`。读到文字时以文字为准并把计数同步过去，所以某次读不到之后仍接得上。`translate(key)` 等价于 `translate_in_context(key, None)`，现有调用方不用改。

**比 macOS 做得更好的两处，有意不照搬：**

- 读不到文字时用计数，嵌套照样成立；macOS 在这种应用里只会出「「「「「。
- 多出来的 `}` 一律出」。macOS 的 `』」` 翻转看不出用处，照搬等于引入一个说不清的状态。

**偏好到引擎的链路（这一刀已接通）：**

- 共享偏好 `punctuation_profile`（`standard` 缺省、`macos_pinyin`），缺省时不写进文档，没有这个键的旧文档照常读；不认识的取值让整份文档解析失败，与 `quote_style` 一致。
- client-core 的 `punctuation_marks()` 把它带进解析结果，View 的 `punctuation_marks` 只在非缺省时多一个 `profile` 字段，缺省时 JSON 与之前逐字相同，老宿主和既有测试不受影响。
- host-api 的 `apply_punctuation_marks` 填 `EngineOptions.macos_pinyin_punctuation`，引擎在建会话和空闲重建时换上档案。
- `~` `|` `$` 不需要宿主改动：运行时把所有 ASCII 标点键都交给引擎的转换。

**macOS 宿主（这一刀只动一处）：** 普通中文方案原来在 `InputController.mm` 里把 `Shift+[` `Shift+]` 写死成「」，并自己补右半个。View 报告档案是 `macos_pinyin` 时，这一段改为调用引擎的中文标点路线，由引擎按深度出「『』」，宿主不补右半个，也不跨过——macOS 自带输入法同样不补。引擎没接手时落回原来的路线。

**这一刀不做的事（后续另起切片）：**

1. 设置入口。共享 React 设置页和 macOS 进程内设置窗口都没有这一项，目前只能在 `preferences.json` 里加 `"punctuation_profile": "macos_pinyin"`（macOS 在 `~/Library/Application Support/app.msime.macos/`）。
2. 契约 `shared/contracts/punctuation/policy.json` 还没有收录这个档案。其余宿主接入时再升版本、改 `generate.py`、同步 `policy.h` 和 `shared/contracts/punctuation/README.md` 里列的各宿主手抄表。
3. 英文模式下的中文标点（`shared/input/EnglishModeOutput.h`）、空格转回 ASCII 的反查表、Windows、Linux、移动端的手抄表都没有认这个档案。
4. 宿主把光标前一段文字传给引擎。macOS 宿主现在只读一个字符（`MSIMETextClientPrecedingUnicodeScalar`），所以档案目前只靠内部计数，`translate_in_context` 还没有调用方；回看多远需要实测。
5. 账号同步键。

## Alternatives considered

- **逐字复刻 macOS，只看光标前一个字符。** 最强的理由是实现最小，宿主已经能读一个字符，而且与 macOS 的行为一一对应。没采用：一个字符的规则打不出带正文的嵌套，`他说{你好{` 的第二个 `{` 前面是汉字，只会再出「；macOS 自己显然回看得更远。
- **只用内部计数，不读文字。** 最强的理由是完全不改宿主，三个桌面平台和移动端的行为一致。没采用：引擎里的标点状态回退不了，`engine-wasm` 的注释已经记过同样的问题（空闲退格删掉 `“` 后引擎下一次仍会给 `”`）；用户删掉括号后计数就一直偏高。计数只留作读不到文字时的兜底。
- **给 `quote_style` 加一个取值。** 最强的理由是不引入新枚举。没采用：`quote_style` 管的是 `'` `"` 两个键，这个档案动的是 `{` `}` `~` `|` `$`，把它们塞进同一个枚举，会让引号样式和档案互相覆盖，设置页也解释不清。
- **任意逐键映射表。** `2026-10-13-punctuation-mark-styles.md` 已经权衡过并否掉，理由依然成立：宿主的补全、跨过、空格转换都要改成运行时数据，Windows TIP 还要让整张表过 IPC。这个档案不是第三组需要切换的字，不触发那篇笔记里的重访条件。

## Acceptance criteria

- `cargo test -p msime-engine --locked` 通过；`Standard` 档案下的既有用例逐字不变。
- 新增用例覆盖 macOS 上实测的序列：`{{}}`、`{}{}`、`{{{{}}}}`、单个 `}`、`}{}{`，以及 `~` `|` 两个单键。
- 带文字的用例覆盖正文隔开的嵌套、不配对的收尾字符、手打的「算一层，以及读到文字后计数重新对齐。
- 偏好往返、View 报告和 host-api 端到端用例通过：打开档案后 `~` `|` 出 `～` `｜`，`{{}}{}` 得到 `「『』」「」`，改回缺省后 `~` 重新交回宿主。
- 没有设置 `punctuation_profile` 的用户，行为和 View 的 JSON 逐字不变。
- macOS 宿主的改动需要在 macOS 上编译并手动按键验证，这个容器做不了（见 Risks）。

## Risks

- **没有宿主传文字之前，档案只有计数这一条路。** 计数偏高的问题在这一阶段是存在的；好在档案还没有任何入口，不会影响现有用户。
- **回看多远没有实测。** `open_corner_quotes` 扫的是宿主给的全部文字，宿主传多长由接入时决定。窗口太短会漏掉远处还没合上的「，窗口太长则每次按 `{` `}` 都要扫一遍；该键位出现频率低，成本可以接受，但要给窗口定上限。
- **macOS 宿主改动没有编译过。** `InputController.mm` 的改动只有一个小分支和一个读 View 的函数，但这个容器不是 macOS，Objective-C++ 和 `ShortcutTest.mm` 都没跑过，需要在 macOS runner 或本机上编译并按键验证。
- **档案下 `{` 不补右半个。** 这对齐 macOS 自带输入法，但与同一台机器上其他方案里「`{` 补 `}`」的习惯不同；「成对标点」开关在档案下对 `{` `}` 不起作用。
- **其余宿主的手抄表。** 契约、成对补全和各平台的反查表都还没动，这些要在各平台的 CI 上验证。

## 相关笔记

- [引号样式与方头括号样式](../../implemented/feature/2026-10-13-punctuation-mark-styles.md)：部分重叠。那篇定下样式如何从偏好经解析传到引擎和各宿主，这个档案沿用同一条链路；那篇被否的「任意逐键映射表」在这里没有被重新打开。
