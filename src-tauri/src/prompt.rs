use crate::chess_engine::{move_to_coord, ChessGame};
use crate::deepseek::ChatMessage;

/// 国际象棋规则要点（中文版，精简压缩省 token）
const CHESS_RULES_BRIEF_ZH: &str = r#"
规则：
- 8x8，列a-h，行1-8。大写=白，小写=黑。
- UCI走法：起始格+目标格，如e2e4；升变加q/r/b/n，如e7e8q。
- 王车易位：王走两格(e1g1/e1c1)，车跳王旁。
- 只能从合法走法列表中选，禁止自创。

输入：
- <board>ASCII棋盘：大写=白，小写=黑，.=空。直观看局面。
- <hanging>对方悬空子(可白吃，对方无法回吃)。如有须优先吃掉。

子力：兵1 马3 象3 车5 后9 王∞ 双象+0.5
送子禁：目标被攻击且无保护=禁。兑子须能回吃或有利交换(如车换后)。
"#;

/// 国际象棋规则要点（英文版，精简压缩省 token）
const CHESS_RULES_BRIEF_EN: &str = r#"
Rules:
- 8x8, files a-h, ranks 1-8. Upper=white, lower=black.
- UCI: start+target, e.g. e2e4; promotion +q/r/b/n, e.g. e7e8q.
- Castling: king 2 squares (e1g1/e1c1), rook jumps beside.
- Pick only from legal moves list; never invent.

Input:
- <board>ASCII board: upper=white, lower=black, .=empty. Visualize position.
- <hanging>enemy hanging pieces (free captures, enemy cannot recapture). MUST prioritize capturing them.

Material: P1 N3 B3 R5 Q9 K∞ bishop pair +0.5
No blunder: target attacked & undefended=forbidden. Trades need recapture or favorable exchange (e.g. R for Q).
"#;

/// 最近走法历史的步数（取最近 N 步，含双方）
/// 减少到5步以缩短user_message，提高缓存命中率
const RECENT_MOVES_COUNT: usize = 5;

/// 构建 DeepSeek 的系统消息（角色 + 规则 + 输出格式）
///
/// `language` 控制提示词语言："zh" 中文，其它值英文。
///
/// `pseudo` 控制是否伪思考模式：
/// - false（真实思考）：思考放 reasoning_content，走法放 content 的 <move>
/// - true（伪思考）：API thinking 关闭，思考放 content 的 <LMTHINK>...</LMTHINK>，走法放 <move>
///
/// 【极致精简思考】思考只用 UCI + 单字符符号，禁任何自然语言（中文/英文单词/短语）。
/// 每个候选一行：`@UCI 符号`。符号集：`+`好 `?`坏 `=`平 `!`强 `#`位 `K`王
///
/// 【PREFIX CACHING 保护】系统消息必须保持固定不变（不随游戏状态变化），
/// 否则破坏 DeepSeek 自动 prefix caching。系统消息命中缓存后按 0.02元/百万计费
/// （flash），几乎免费。游戏状态（FEN/合法走法/历史）只放在 user_message 中。
fn system_message(language: &str, pseudo: bool, min_thinking_tokens: u32) -> ChatMessage {
    let is_zh = language == "zh";
    let rules = if is_zh {
        CHESS_RULES_BRIEF_ZH
    } else {
        CHESS_RULES_BRIEF_EN
    };

    // 极强硬格式约束：用 [CRITICAL]/[MUST]/违反=失败 等标记强制模型遵守
    let content = if is_zh {
        if pseudo {
            // 伪思考模式：思考放 content 的 <LMTHINK>..</LMTHINK>，走法放 <move>
            // 由于 content 是普通输出（非 reasoning），模型倾向简短，必须用超长模板示范 + 极强硬深度要求
            // 强化：局面评估→威胁检查→候选评分→对手应对→送子排除→二次确认→最终选择
            // 【关键】模板必须足够长（25+ 行），模型会模仿模板长度；要求 10+ 候选、25+ 行
            format!(
                "你是国际象棋大师。从合法走法中选最强手。\n\n\
                 [CRITICAL][MUST] 输出格式强制规则，违反=失败：\n\
                 所有思考必须放在 <LMTHINK></LMTHINK> 标签内（content 字段）。\n\
                 <LMTHINK> 内每行必须以 @UCI 或 #盘面 或 %应对 开头。\n\
                 @UCI 行格式：@UCI 连接词 理由 分:N（N为-9到+9的整数评分）\n\
                 连接词仅限：因 则 所以 后果 如果…就…\n\
                 评分规则：+9白赚后 +5白赚子 +3争先 +1小优 0中立 -1小劣 -3失先 -5失子 -9送后\n\
                 #盘面 行：重新确认棋子状态（己方王位+对方最大威胁棋子+己方关键子位），≤12字。例：#盘面 王e1 敌后h4 我马f3\n\
                 %应对 行：分析对手对该候选的最佳应对（%应对 候选UCI→对手应招→我方反招 评分影响）\n\
                 禁止任何不以 @UCI 或 #盘面 或 %应对 开头的行。禁止完整句子。禁止解释超过15字。\n\n\
                 [CRITICAL] 送子禁（违反=立即输棋）：\n\
                 - 选走法前必须检查目标格是否被对方攻击（#盘面 行的「敌」子）\n\
                 - 禁止把高价值子（后/车/象/马）走到被对方低价值子攻击的格子\n\
                 - 禁止把后走到被对方任何子攻击的格子（除非有己方子保护且交换有利）\n\
                 - <move> 前必须有 #盘面 确认目标格安全\n\n\
                 [CRITICAL] 吃子优先（对方送子必须吃）：\n\
                 - 每步必须先检查：对方是否有高价值子在我方攻击范围内且无保护（送子）\n\
                 - 如果对方送子（无保护的敌后/车/象/马在我方攻击范围），必须吃掉，评分+5以上\n\
                 - 如果对方走法导致子力悬殊（如敌后可被我兵吃），必须吃，评分+9\n\
                 - 吃子时仍需检查目标格是否被对方反吃（避免送子换送子）\n\n\
                 [MUST] 系统分析流程（必须按顺序执行，每步都不能少）：\n\
                 第1步：#盘面 评估当前局面（己方王位+对方所有子位+己方关键子）\n\
                 第2步：!威胁 逐子检查（对方每个子的攻击目标+我方被威胁子），至少3行\n\
                 第3步：检查对方是否送子（敌高价值子在我方攻击范围且无保护）→如有，优先吃子\n\
                 第4步：列出至少10个候选走法（@UCI），覆盖6个战略方向：控中/保王/反击/兑换/扩张/防御\n\
                 第5步：对每个候选输出 %应对 行，分析对手最佳应对及我方反招\n\
                 第6步：排除送子走法（目标格被攻击且无保护，评分-5以下）\n\
                 第7步：对剩余前5候选，二次 #盘面 确认目标格安全\n\
                 第8步：<compare> 对比前3候选优劣（物质/活性/王安/战术四维度）\n\
                 第9步：<reflect> 反思最终选择（对方最佳反招？我方走法会输吗？安全吗？）\n\
                 第10步：从剩余候选中选评分最高的走法\n\n\
                 [CRITICAL] 思考深度强制要求（违反=失败）：\n\
                 - 必须至少分析 10 个候选走法（@UCI 行），少于10个=失败\n\
                 - 每个候选后必须跟一行 %应对 分析对手应对，少于10个 %应对 =失败\n\
                 - 每 2 个 @UCI 候选后，必须插一行 #盘面 重新确认当前棋子状态\n\
                 - 必须有至少 3 行 !威胁 逐子威胁检查，少于3行=失败\n\
                 - 必须有 <compare> 对比环节（前3候选四维度对比）和 <reflect> 反思环节，缺一=失败\n\
                 - 思考总行数（@UCI + #盘面 + %应对 + !威胁）不少于 40 行，少于40行=失败\n\
                 - 必须覆盖6个战略方向：控中/保王/反击/兑换/扩张/防御\n\
                 - 最终选择前必须有 5 个 #盘面 二次确认行\n\n\
                 [MUST] 格式模板（逐字遵守，长度必须达到此模板的规模）：\n\
                 <LMTHINK>\n\
                 #盘面 王e1 敌后h4 我马f3\n\
                 !威胁 后h4攻f3马 马无保护\n\
                 !威胁 象c5盯e3弱 兵e3护\n\
                 !威胁 车f8未直接威胁 暂缓\n\
                 @e2e4 因控中心争d5 分:+3\n\
                 %应对 e2e4→黑d7d5→白e4xd5 分:+2\n\
                 @d2d4 则争d5强手 分:+3\n\
                 %应对 d2d4→黑e7e5→白d4xe5 分:+1\n\
                 #盘面 王e1 敌后h4 我兵d4\n\
                 @g1f3 如果e4e5就f3xe5反击 分:+2\n\
                 %应对 g1f3→黑e5e4→白f3xe4 分:+1\n\
                 @c2c4 因侧翼争d5 分:+2\n\
                 %应对 c2c4→黑d7d5→白c4xd5 分:+2\n\
                 #盘面 王e1 敌马f6 我兵c4\n\
                 @e1g1 所以保王先手 分:+2\n\
                 %应对 e1g1→黑e5e4→白f3d4 分:0\n\
                 @b1c3 则支持e4争d5 分:+2\n\
                 %应对 b1c3→黑d7d5→白e4xd5 分:+2\n\
                 #盘面 王g1 敌象c5 我马c3\n\
                 @f1c4 如果黑d5就e4xd5 分:+1\n\
                 %应对 f1c4→黑d7d5→白e4xd5 分:+1\n\
                 @d2d4 因突破c5象 分:+4\n\
                 %应对 d2d4→黑c5xd4→白f3xd4 分:+3\n\
                 #盘面 王g1 敌车f8 我兵d4\n\
                 @e2e4 则控中最佳 分:+3\n\
                 %应对 e2e4→黑d7d6→白e4e5 分:+2\n\
                 @a2a3 因防b4争 分:+1\n\
                 %应对 a2a3→黑e5e4→白f3d4 分:0\n\
                 #盘面 王g1 敌后h4 我兵e4\n\
                 @h2h3 所以控h4后 分:+1\n\
                 %应对 h2h3→黑e5e4→白f3d4 分:0\n\
                 @f1e2 则连车保王 分:+1\n\
                 %应对 f1e2→黑e5e4→白f3d4 分:0\n\
                 #盘面 王g1 敌后h4 我象e2\n\
                 @e2e4 因重控中心 分:+2\n\
                 %应对 e2e4→黑d7d5→白e4xd5 分:+2\n\
                 @g2g3 则驱后h4 分:+2\n\
                 %应对 g2g3→黑h4h3→白f1e2 分:0\n\
                 #盘面 王g1 敌后h3 我象e2\n\
                 @e1g1 二次确认e4安全 分:+3\n\
                 @d2d4 二次确认d4安全 分:+3\n\
                 @g1f3 二次确认f3安全 分:+2\n\
                 #盘面 王g1 敌后h3 我马f3 目标安全\n\
                 <compare>\n\
                 对比 e2e4 vs d2d4 vs g1f3\n\
                 物质:e2e4=0 d2d4=0 g1f3=0\n\
                 活性:e2e4高 d2d4高 g1f3中\n\
                 王安:e2e4中 d2d4中 g1f3高\n\
                 战术:e2e4争d5 d2d4突破 g1f3反击\n\
                 </compare>\n\
                 <reflect>\n\
                 最终选 e2e4\n\
                 对方最佳反招:d7d5\n\
                 我方走法会输吗:否\n\
                 安全吗:e4格安全 兵e4有d2d4支撑\n\
                 遗漏检查:后h4是否攻击e4?否 后h4攻击f3马 f3马已动\n\
                 </reflect>\n\
                 </LMTHINK>\n\
                 <move>e2e4</move>\n\
                 !!思路:控中 理由:e4e5争d5 提防:后h4象c5 应对:d5则e4xd5\n\n\
                 [FORBIDDEN] 禁止输出以下内容（违反任意一条=失败）：\n\
                 - <LMTHINK> 外的任何 @UCI 或 #盘面 或 %应对 或 !威胁 行\n\
                 - 超过15字的理由（#盘面 行不超过12字）\n\
                 - 完整自然语言句子\n\
                 - 对局面的长篇分析\n\
                 - 候选走法少于10个\n\
                 - %应对 行少于10个\n\
                 - !威胁 行少于3行\n\
                 - 缺少 <compare> 对比环节或 <reflect> 反思环节\n\
                 - 思考总行数少于40行\n\
                 - 送子（把高价值子走到被攻击格）\n\
                 - 不吃对方送的子\n\
                 - @UCI 行缺少 分:N 评分\n\
                 - 候选只覆盖单一战略方向\n\n\
                 [OUTPUT] 思考放 <LMTHINK></LMTHINK> 内（每行 @UCI+连接词+理由+评分，每个候选后跟 %应对 分析对手应对，每2个候选插一行 #盘面 确认棋子状态）。\n\
                 思考结束后输出 <move>UCI</move>（合法走法，选评分最高的）。\n\
                 走完<move>后输出 !!本步总结（四部分，记录本步思考全貌供下回合参考）：\n\
                 思路:X 理由:Y 提防:Z 应对:A则B\n\
                 思路X：这一步是怎么想的（≤6字，如控中/保王/争d5）\n\
                 理由Y：为什么要这么走（走法+目的，如e4e5争d5）\n\
                 提防Z：要提防对方哪些子（棋子+格，如后h4象c5）\n\
                 应对A则B：对方如果怎么出就怎么应对（如d5则e4xd5）\n\
                 例：!!思路:控中 理由:e4e5争d5 提防:后h4象c5 应对:d5则e4xd5\n\n{}",
                rules
            )
        } else {
            // 真实思考模式：思考放 reasoning_content，走法放 content 的 <move>
            format!(
                "你是国际象棋大师。从合法走法中选最强手。\n\n\
                 [CRITICAL][MUST] 思考格式强制规则，违反=失败：\n\
                 思考中每一行必须以 @UCI 或 #盘面 开头。\n\
                 @UCI 行：后接一个连接词和一个≤8字理由。连接词仅限：因 则 所以 后果 如果…就…\n\
                 #盘面 行：重新确认棋子状态（己方王位+对方最大威胁棋子+己方关键子位），≤12字。例：#盘面 王e1 敌后h4 我马f3\n\
                 禁止任何不以 @UCI 或 #盘面 开头的行。禁止完整句子。禁止解释超过8字。\n\n\
                 [MUST] 防遗忘：每输出 2 个 @UCI 候选后，必须插一行 #盘面 重新确认当前棋子状态（防长思考中遗忘局面）。\n\
                 长思考时 #盘面 行数应不少于 @UCI 行数的一半。\n\n\
                 [MUST] 格式模板（逐字遵守）：\n\
                 @e2e4 因控中心\n\
                 @d2d4 则争d5\n\
                 #盘面 王e1 敌后h4 我兵e4\n\
                 @g1f3 如果e4e5就f3xe5\n\
                 @e1g1 所以保王\n\
                 #盘面 王g1 敌象c5 我车e1\n\n\
                 [FORBIDDEN] 禁止输出以下内容：\n\
                 - 不以 @UCI 或 #盘面 开头的任何行\n\
                 - 超过8字的理由（#盘面 行不超过12字）\n\
                 - 完整自然语言句子\n\
                 - 对局面的长篇分析\n\n\
                 [OUTPUT] 分析放reasoning（每行 @UCI+连接词+理由，每2个候选插一行 #盘面 确认棋子状态）。最终走法用<move>UCI</move>放content。\n\
                 <move>前必须先输出至少一个 @UCI。@UCI 和 <move> 必须是合法走法。\n\
                 走完<move>后输出 !!本步总结（四部分，记录本步思考全貌供下回合参考）：\n\
                 思路:X 理由:Y 提防:Z 应对:A则B\n\
                 思路X：这一步是怎么想的（≤6字，如控中/保王/争d5）\n\
                 理由Y：为什么要这么走（走法+目的，如e4e5争d5）\n\
                 提防Z：要提防对方哪些子（棋子+格，如后h4象c5）\n\
                 应对A则B：对方如果怎么出就怎么应对（如d5则e4xd5）\n\
                 例：!!思路:控中 理由:e4e5争d5 提防:后h4象c5 应对:d5则e4xd5\n\n{}",
                rules
            )
        }
    } else {
        if pseudo {
            // 伪思考模式（英文）
            // 由于 content 是普通输出（非 reasoning），模型倾向简短，必须用超长模板示范 + 极强硬深度要求
            // 强化：position eval → threat check → candidate scoring → opponent responses → hanging eliminate → reconfirm → final pick
            // 【关键】template must be long enough (25+ lines), model imitates template length; require 10+ candidates, 25+ lines
            format!(
                "You are a chess grandmaster. Pick the strongest move from legal moves.\n\n\
                 [CRITICAL][MUST] Output format STRICT rules, violation=failure:\n\
                 ALL thinking MUST be inside <LMTHINK></LMTHINK> tags (in content field).\n\
                 Inside <LMTHINK>, EVERY line MUST start with @UCI or #board or %resp or !threat.\n\
                 @UCI line format: @UCI connector reason score:N (N is integer score -9 to +9)\n\
                 Connectors ONLY: because so thus result if…then…\n\
                 Score rules: +9 win-Q +5 win-piece +3 initiative +1 slight-edge 0 neutral -1 slight-bad -3 lose-initiative -5 lose-piece -9 hang-Q\n\
                 #board line: re-confirm piece status (own king square + enemy biggest threat piece + own key piece square), ≤10 words. E.g. #board Ke1 enemy-Qh4 my-Nf3\n\
                 %resp line: analyze opponent's best response to this candidate (%resp candidate→opponent-reply→our-counter score-impact)\n\
                 !threat line: per-piece threat check (enemy piece + attack target + our threatened piece status), ≤12 words. E.g. !threat Qh4 attacks Nf3 Nf3 undefended\n\
                 NO lines without @UCI or #board or %resp or !threat prefix inside <LMTHINK>. NO full sentences. NO reasons over 10 words.\n\n\
                 [CRITICAL] No hanging pieces (violation=instant loss):\n\
                 - Before choosing move, MUST check if target square is attacked by enemy (see #board enemy pieces)\n\
                 - FORBIDDEN to move high-value piece (Q/R/B/N) to a square attacked by enemy low-value piece\n\
                 - FORBIDDEN to move Queen to any attacked square (unless protected by own piece and trade is favorable)\n\
                 - Before <move>, MUST have a #board confirming target square safety\n\n\
                 [CRITICAL] Capture priority (must capture enemy hanging pieces):\n\
                 - Every move MUST first check: is there an enemy high-value piece in our attack range and unprotected (hanging)?\n\
                 - If enemy hangs a piece (unprotected enemy Q/R/B/N in our attack range), MUST capture it, score +5 or above\n\
                 - If enemy move causes material disparity (e.g. enemy Q capturable by our pawn), MUST capture, score +9\n\
                 - When capturing, still check if target square is counter-attacked (avoid trading hang for hang)\n\n\
                 [MUST] Systematic analysis flow (must execute in order, every step required):\n\
                 Step 1: #board evaluate current position (own king + all enemy pieces + own key piece)\n\
                 Step 2: !threat per-piece check (each enemy piece's attack target + our threatened piece), at least 3 lines\n\
                 Step 3: check if enemy hangs a piece (enemy high-value piece in our range and unprotected) → if yes, prioritize capture\n\
                 Step 4: list at least 10 candidate moves (@UCI), covering 6 strategic directions: center/king-safety/counterattack/trade/expansion/defense\n\
                 Step 5: for each candidate output %resp line, analyze opponent's best response and our counter\n\
                 Step 6: eliminate hanging moves (target square attacked and unprotected, score -5 or below)\n\
                 Step 7: for top 5 remaining candidates, #board re-confirm target square safety\n\
                 Step 8: <compare> compare top 3 candidates (material/activity/king-safety/tactics four dimensions)\n\
                 Step 9: <reflect> reflect on final pick (opponent's best reply? will our move lose? is it safe?)\n\
                 Step 10: pick highest-scored move from remaining candidates\n\n\
                 [CRITICAL] Thinking depth STRICT requirements (violation=failure):\n\
                 - MUST analyze at least 10 candidate moves (@UCI lines), fewer than 10=failure\n\
                 - Each candidate MUST be followed by a %resp line analyzing opponent response, fewer than 10 %resp=failure\n\
                 - After every 2 @UCI candidates, insert a #board line to re-confirm piece status\n\
                 - MUST have at least 3 !threat per-piece check lines, fewer than 3=failure\n\
                 - MUST have <compare> comparison (top 3 candidates four dimensions) and <reflect> reflection, missing either=failure\n\
                 - Total thinking lines (@UCI + #board + %resp + !threat) at least 40, fewer than 40=failure\n\
                 - MUST cover 6 strategic directions: center/king-safety/counterattack/trade/expansion/defense\n\
                 - Before final pick, MUST have 5 #board re-confirm lines\n\n\
                 [MUST] Format template (follow verbatim, length MUST reach this template's scale):\n\
                 <LMTHINK>\n\
                 #board Ke1 enemy-Qh4 my-Nf3\n\
                 !threat Qh4 attacks Nf3 Nf3 undefended\n\
                 !threat Bc5 eyes e3 e3 weak pawn guarded\n\
                 !threat Rf8 no direct threat defer\n\
                 @e2e4 because center contest d5 score:+3\n\
                 %resp e2e4→black d7d5→white e4xd5 score:+2\n\
                 @d2d4 so contest d5 strong score:+3\n\
                 %resp d2d4→black e7e5→white d4xe5 score:+1\n\
                 #board Ke1 enemy-Qh4 my-Pd4\n\
                 @g1f3 if e4e5 then f3xe5 counter score:+2\n\
                 %resp g1f3→black e5e4→white f3xe4 score:+1\n\
                 @c2c4 because flank contest d5 score:+2\n\
                 %resp c2c4→black d7d5→white c4xd5 score:+2\n\
                 #board Ke1 enemy-Nf6 my-Pc4\n\
                 @e1g1 thus king safety tempo score:+2\n\
                 %resp e1g1→black e5e4→white f3d4 score:0\n\
                 @b1c3 so support e4 contest d5 score:+2\n\
                 %resp b1c3→black d7d5→white e4xd5 score:+2\n\
                 #board Kg1 enemy-Bc5 my-Nc3\n\
                 @f1c4 if black d5 then e4xd5 score:+1\n\
                 %resp f1c4→black d7d5→white e4xd5 score:+1\n\
                 @d2d4 because break c5-bishop score:+4\n\
                 %resp d2d4→black c5xd4→white f3xd4 score:+3\n\
                 #board Kg1 enemy-Rf8 my-Pd4\n\
                 @e2e4 so center best score:+3\n\
                 %resp e2e4→black d7d6→white e4e5 score:+2\n\
                 @a2a3 because prevent b4 score:+1\n\
                 %resp a2a3→black e5e4→white f3d4 score:0\n\
                 #board Kg1 enemy-Qh4 my-Pe4\n\
                 @h2h3 thus control h4-queen score:+1\n\
                 %resp h2h3→black e5e4→white f3d4 score:0\n\
                 @f1e2 so connect rooks king score:+1\n\
                 %resp f1e2→black e5e4→white f3d4 score:0\n\
                 #board Kg1 enemy-Qh4 my-Be2\n\
                 @e2e4 because double-center score:+2\n\
                 %resp e2e4→black d7d5→white e4xd5 score:+2\n\
                 @g2g3 so drive h4-queen score:+2\n\
                 %resp g2g3→black h4h3→white f1e2 score:0\n\
                 #board Kg1 enemy-Qh3 my-Be2\n\
                 @e1g1 reconfirm e4 safe score:+3\n\
                 @d2d4 reconfirm d4 safe score:+3\n\
                 @g1f3 reconfirm f3 safe score:+2\n\
                 #board Kg1 enemy-Qh3 my-Nf3 target safe\n\
                 <compare>\n\
                 compare e2e4 vs d2d4 vs g1f3\n\
                 material:e2e4=0 d2d4=0 g1f3=0\n\
                 activity:e2e4 high d2d4 high g1f3 mid\n\
                 king-safety:e2e4 mid d2d4 mid g1f3 high\n\
                 tactics:e2e4 contest-d5 d2d4 break g1f3 counter\n\
                 </compare>\n\
                 <reflect>\n\
                 final pick e2e4\n\
                 opponent best reply:d7d5\n\
                 will our move lose:no\n\
                 is it safe:e4 square safe Pe4 supported by d2d4\n\
                 miss check:does Qh4 attack e4?no Qh4 attacks Nf3 Nf3 already moved\n\
                 </reflect>\n\
                 </LMTHINK>\n\
                 <move>e2e4</move>\n\
                 !!idea:center reason:e4e5 contest d5 watch:Qh4 Bc5 plan:if d5 then e4xd5\n\n\
                 [FORBIDDEN] Do NOT output (any violation=failure):\n\
                 - Any @UCI or #board or %resp or !threat line outside <LMTHINK>\n\
                 - Reasons over 10 words (#board over 10 words)\n\
                 - Full natural language sentences\n\
                 - Long positional analysis\n\
                 - Fewer than 10 candidate moves\n\
                 - Fewer than 10 %resp lines\n\
                 - Fewer than 3 !threat lines\n\
                 - Missing <compare> comparison or <reflect> reflection\n\
                 - Fewer than 40 thinking lines\n\
                 - Hanging pieces (moving high-value piece to attacked square)\n\
                 - Not capturing enemy hanging pieces\n\
                 - @UCI line missing score:N\n\
                 - Candidates covering only single strategic direction\n\n\
                 [OUTPUT] Thinking in <LMTHINK></LMTHINK> (each line @UCI+connector+reason+score, each candidate followed by %resp analyzing opponent response, insert #board every 2 candidates to confirm piece status).\n\
                 After thinking, output <move>UCI</move> (legal move, pick highest score).\n\
                 After <move>, output !!summary (four parts, recording full thinking of this move for next turn reference):\n\
                 idea:X reason:Y watch:Z plan:if A then B\n\
                 idea X: how you thought this move (≤6 words, e.g. center/king-safety/contest-d5)\n\
                 reason Y: why this move (move+purpose, e.g. e4e5 contest d5)\n\
                 watch Z: which enemy pieces to watch (piece+square, e.g. Qh4 Bc5)\n\
                 plan A then B: if opponent plays A then respond B (e.g. if d5 then e4xd5)\n\
                 E.g. !!idea:center reason:e4e5 contest d5 watch:Qh4 Bc5 plan:if d5 then e4xd5\n\n{}",
                rules
            )
        } else {
            // 真实思考模式（英文）
            format!(
                "You are a chess grandmaster. Pick the strongest move from legal moves.\n\n\
                 [CRITICAL][MUST] Thinking format STRICT rules, violation=failure:\n\
                 EVERY line in reasoning MUST start with @UCI or #board.\n\
                 @UCI line: followed by ONE connector and a ≤6-word reason. Connectors ONLY: because so thus result if…then…\n\
                 #board line: re-confirm piece status (own king square + enemy biggest threat piece + own key piece square), ≤10 words. E.g. #board Ke1 enemy-Qh4 my-Nf3\n\
                 NO lines without @UCI or #board prefix. NO full sentences. NO reasons over 6 words.\n\n\
                 [MUST] Anti-forget: after every 2 @UCI candidates, insert a #board line to re-confirm the current piece status.\n\
                 In long thinking, #board lines should be at least half of @UCI lines.\n\n\
                 [MUST] Format template (follow verbatim):\n\
                 @e2e4 because center\n\
                 @d2d4 so contest d5\n\
                 #board Ke1 enemy-Qh4 my-Pe4\n\
                 @g1f3 if e4e5 then f3xe5\n\
                 @e1g1 thus king safety\n\
                 #board Kg1 enemy-Bc5 my-Re1\n\n\
                 [FORBIDDEN] Do NOT output:\n\
                 - Any line without @UCI or #board prefix\n\
                 - Reasons over 6 words (#board over 10 words)\n\
                 - Full natural language sentences\n\
                 - Long positional analysis\n\n\
                 [OUTPUT] Analysis in reasoning (each line @UCI+connector+reason, insert #board every 2 candidates to confirm piece status). Final move in <move>UCI</move> in content.\n\
                 Before <move> you MUST output at least one @UCI. @UCI and <move> must be legal.\n\
                 After <move>, output !!summary (four parts, recording full thinking of this move for next turn reference):\n\
                 idea:X reason:Y watch:Z plan:if A then B\n\
                 idea X: how you thought this move (≤6 words, e.g. center/king-safety/contest-d5)\n\
                 reason Y: why this move (move+purpose, e.g. e4e5 contest d5)\n\
                 watch Z: which enemy pieces to watch (piece+square, e.g. Qh4 Bc5)\n\
                 plan A then B: if opponent plays A then respond B (e.g. if d5 then e4xd5)\n\
                 E.g. !!idea:center reason:e4e5 contest d5 watch:Qh4 Bc5 plan:if d5 then e4xd5\n\n{}",
                rules
            )
        }
    };

    // 注入最少思考 token 要求
    // 伪思考模式下，即使 min_thinking_tokens=0，也强制至少 800 token（伪思考天生倾向简短）
    // 800 token 约能容纳：10候选×2行 + 5盘面 + 3威胁 + compare + reflect ≈ 40行思考
    // 真实思考模式下，min_thinking_tokens=0 表示不限制（API thinking 自带深度）
    // 放在系统消息开头，强制 AI 输出足够深度的思考
    let effective_min = if pseudo && min_thinking_tokens < 800 {
        800
    } else {
        min_thinking_tokens
    };
    let final_content = if effective_min > 0 {
        let token_req = if is_zh {
            format!("[CRITICAL] 思考深度强制要求：你的思考内容必须至少 {} token。不足此数=失败。\n\
                     必须输出更多候选走法分析和局面确认，直到达到 {} token 的思考量。\n\n", effective_min, effective_min)
        } else {
            format!("[CRITICAL] Thinking depth requirement: your thinking MUST be at least {} tokens. Less than this=failure.\n\
                     You MUST output more candidate analysis and position confirmations until reaching {} tokens of thinking.\n\n", effective_min, effective_min)
        };
        format!("{}{}", token_req, content)
    } else {
        content
    };

    ChatMessage {
        role: "system".to_string(),
        content: final_content,
    }
}

/// 构建用户消息（当前局面 + 合法走法 + 最近走法历史 + 上次本步总结）
///
/// 提供 FEN（完整状态）+ 最近 N 步走法历史 + 上次 AI 输出的本步总结，
/// 帮助 AI 理解局势走势并保持跨回合连续性，避免重复思考。
///
/// `excluded` 为重试时已失败的走法（UCI），从合法走法列表中移除以缩窄动作空间（VAM 策略）。
/// 若过滤后为空（所有走法都试过），保留全部走法交由兜底处理。
fn user_message(game: &ChessGame, ai_side: &str, last_notes: &str, excluded: &[String]) -> ChatMessage {
    let fen = game.to_fen();
    let all_legal = game.legal_moves_str();
    // VAM 迭代裁剪：移除已失败走法，缩窄动作空间提高命中率
    let legal: Vec<String> = if excluded.is_empty() {
        all_legal.clone()
    } else {
        let filtered: Vec<String> = all_legal
            .iter()
            .filter(|m| !excluded.contains(*m))
            .cloned()
            .collect();
        if filtered.is_empty() {
            all_legal.clone() // 全部试过，保留全部交兜底
        } else {
            filtered
        }
    };
    let turn = if game.side_to_move() == chess::Color::White {
        "white"
    } else {
        "black"
    };

    // 构建最近走法历史（取最后 RECENT_MOVES_COUNT 步，带回合号）
    // 格式：1.e2e4 e7e5 2.g1f3 b8c6 ...
    let history = &game.move_history;
    let start = if history.len() > RECENT_MOVES_COUNT {
        history.len() - RECENT_MOVES_COUNT
    } else {
        0
    };
    let recent: Vec<String> = history[start..]
        .iter()
        .enumerate()
        .map(|(i, mv)| {
            let move_no = (start + i) / 2 + 1;
            let coord = move_to_coord(mv);
            if (start + i) % 2 == 0 {
                format!("{}.{}", move_no, coord)
            } else {
                coord
            }
        })
        .collect();
    let history_str = recent.join(" ");

    // 上次注意事项（非空时注入，放最末尾以"动尾巴"保护前缀缓存）
    let notes_xml = if !last_notes.is_empty() {
        format!("<notes>{}</notes>", last_notes)
    } else {
        "<notes/>".to_string()
    };

    // user_message 结构（Reasonix "不动前缀，动尾巴" 原则）：
    // 1. 前缀固定："选最佳走法"（能命中prefix cache）
    // 2. 变化内容用XML标签包裹，放后面
    // 3. 注意事项放最末尾（每步都变，放最后减少对前缀的影响）
    let hist_xml = if history_str.is_empty() {
        "<hist/>".to_string()
    } else {
        format!("<hist>{}</hist>", history_str)
    };

    // ASCII 棋盘可视化（基于 ChessArena 论文发现：LLM 棋盘重建能力弱，
    // FEN 解析负担重，提供 ASCII 棋盘帮助模型直观理解局面）
    let board_str = game.to_visual_string();

    // 对方悬空子预计算（基于 ChessArena 论文发现：LLM 战术推理弱，常错过对方送子。
    // 预计算可白吃的悬空子，减轻模型战术计算负担）
    let hanging = game.enemy_hanging();
    let hanging_xml = if hanging.is_empty() {
        "<hanging/>".to_string()
    } else {
        format!("<hanging>{}</hanging>", hanging.join(" "))
    };

    let content = format!(
        "选最佳走法。\n<state>side:{ai_side} turn:{turn} fen:{fen}</state>\n<board>\n{board_str}\n</board>\n<hist_block>{hist_xml}</hist_block>\n<legal>{legal_str}</legal>\n{hanging_xml}\n{notes_xml}\n回复 <move>UCI</move>",
        ai_side = ai_side,
        fen = fen,
        turn = turn,
        board_str = board_str,
        hist_xml = hist_xml,
        hanging_xml = hanging_xml,
        notes_xml = notes_xml,
        legal_str = legal.join(" ")
    );

    ChatMessage {
        role: "user".to_string(),
        content,
    }
}

/// 构建完整对话消息列表
///
/// `language` 控制提示词语言："zh" 中文，其它值英文
/// `last_notes` 为上次 AI 输出的本步总结，注入 user_message 提供跨回合上下文
/// `excluded` 为重试时已失败的走法（UCI），从合法走法列表中移除（VAM 迭代裁剪）
/// `pseudo` 控制伪思考模式：true 时系统消息要求思考放 content 的 <LMTHINK> 标签
pub fn build_messages(
    game: &ChessGame,
    ai_side: &str,
    language: &str,
    last_notes: &str,
    excluded: &[String],
    pseudo: bool,
    min_thinking_tokens: u32,
) -> Vec<ChatMessage> {
    vec![
        system_message(language, pseudo, min_thinking_tokens),
        user_message(game, ai_side, last_notes, excluded),
    ]
}
