// Lichess 官方音效播放器（MP3 资源来自 lichess-org/lila 仓库，AGPL 协议）
//
// 设计要点：
// - 10 个 MP3 文件在构建时由 Vite 打包，运行时通过 URL 加载
// - 每次播放创建新 Audio 对象：避免复用导致的 currentTime 重置冲突和播放中断问题
// - 音量/静音设置持久化到 localStorage（独立于后端 settings，避免频繁改 Rust 端）
// - autoplay 限制：浏览器可能阻止未用户激活的播放，首次播放需在用户点击后

import MoveMp3 from "../assets/sounds/Move.mp3";
import CaptureMp3 from "../assets/sounds/Capture.mp3";
import CheckMp3 from "../assets/sounds/Check.mp3";
import CheckmateMp3 from "../assets/sounds/Checkmate.mp3";
import ConfirmationMp3 from "../assets/sounds/Confirmation.mp3";
import GenericNotifyMp3 from "../assets/sounds/GenericNotify.mp3";
import LowTimeMp3 from "../assets/sounds/LowTime.mp3";
import VictoryMp3 from "../assets/sounds/Victory.mp3";
import DefeatMp3 from "../assets/sounds/Defeat.mp3";
import DrawMp3 from "../assets/sounds/Draw.mp3";

export type SoundName =
  | "move"
  | "capture"
  | "check"
  | "checkmate"
  | "castle"
  | "promote"
  | "gameStart"
  | "lowTime"
  | "victory"
  | "defeat"
  | "draw";

// 音效文件映射。Lichess standard 主题没有 Castle.mp3/Promote.mp3，
// 复用 Move/Confirmation（Lichess 默认行为）。
const SOUND_FILES: Record<SoundName, string> = {
  move: MoveMp3,
  capture: CaptureMp3,
  check: CheckMp3,
  checkmate: CheckmateMp3,
  castle: MoveMp3,
  promote: ConfirmationMp3,
  gameStart: GenericNotifyMp3,
  lowTime: LowTimeMp3,
  victory: VictoryMp3,
  defeat: DefeatMp3,
  draw: DrawMp3,
};

// 预加载的 Audio 对象（用于预热浏览器音频解码，首次播放更快）
const audioCache: Map<SoundName, HTMLAudioElement> = new Map();
const VOLUME_KEY = "chess_sound_volume";
const ENABLED_KEY = "chess_sound_enabled";

/// 预加载所有音效（应用启动时调用一次，加速首次播放）
export function preloadSounds(): void {
  if (typeof window === "undefined") return;
  for (const name of Object.keys(SOUND_FILES) as SoundName[]) {
    const src = SOUND_FILES[name];
    const audio = new Audio(src);
    audio.preload = "auto";
    audio.load();
    audioCache.set(name, audio);
  }
}

/// 播放指定音效。音量/静音从 localStorage 读取。
/// 每次创建新 Audio 对象避免复用冲突（连续快速播放同一音效时旧对象还在播放）
export function playSound(name: SoundName): void {
  if (!getSoundEnabled()) return;
  const src = SOUND_FILES[name];
  if (!src) return;

  const vol = Math.max(0, Math.min(1, getSoundVolume() / 100));
  // 每次创建新 Audio 对象，避免复用导致的播放冲突
  const audio = new Audio(src);
  audio.volume = vol;
  // autoplay 限制：首次未用户交互时浏览器会拒绝，catch 静默处理
  const playPromise = audio.play();
  if (playPromise) {
    playPromise.catch(() => {
      // 播放失败（autoplay 限制）：尝试从缓存的对象播放
      const cached = audioCache.get(name);
      if (cached) {
        cached.volume = vol;
        cached.currentTime = 0;
        cached.play().catch(() => {});
      }
    });
  }
  // 播放结束后释放资源
  audio.addEventListener("ended", () => {
    audio.src = "";
    audio.remove();
  });
}

/// 音量（0-100），默认 70
export function getSoundVolume(): number {
  if (typeof localStorage === "undefined") return 70;
  const v = parseInt(localStorage.getItem(VOLUME_KEY) || "70", 10);
  if (isNaN(v)) return 70;
  return Math.max(0, Math.min(100, v));
}

export function setSoundVolume(volume: number): void {
  if (typeof localStorage === "undefined") return;
  const v = Math.max(0, Math.min(100, Math.round(volume)));
  localStorage.setItem(VOLUME_KEY, String(v));
}

/// 静音开关，默认 false（不静音）
export function getSoundEnabled(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(ENABLED_KEY) !== "false";
}

export function setSoundEnabled(enabled: boolean): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(ENABLED_KEY, String(enabled));
}

/// 根据 UCI 走法判断走法类型
function classifyMove(uci: string): "castle" | "promote" | "normal" {
  if (uci.length === 5) return "promote";
  // 王车易位：王从 e1/e8 走两格到 g1/c1/g8/c8
  if (/^e1[g|c]1$|^e8[g|c]8$/.test(uci)) return "castle";
  return "normal";
}

/// 判断是否吃子：检查目标格在旧 FEN 中是否有棋子
/// toSquare 如 "e4"
function isCaptureMove(oldFen: string, toSquare: string): boolean {
  const file = toSquare.charCodeAt(0) - 97; // 0-7 (a-h)
  const rank = parseInt(toSquare[1], 10) - 1; // 0-7 (1-8)
  if (file < 0 || file > 7 || rank < 0 || rank > 7) return false;
  const boardPart = oldFen.split(" ")[0];
  const rows = boardPart.split("/");
  if (rows.length !== 8) return false;
  // FEN rows 从 rank 8 到 rank 1，rank 0 (即 1) 在 rows[7]
  const row = rows[7 - rank];
  let col = 0;
  for (const ch of row) {
    if (/\d/.test(ch)) {
      col += parseInt(ch, 10);
    } else {
      if (col === file) return true;
      col += 1;
    }
  }
  return false;
}

/// 统一走棋音效播放（人/AI/鳕鱼通用）
///
/// 播放规则（与 Lichess 行为一致）：
/// - castle → castle 音效
/// - promote → promote 音效
/// - 吃子 → capture 音效
/// - 普通走子 → move 音效
/// - 将军（非终局）→ 追加 check 音效
/// - 将杀 → checkmate 音效（覆盖走子音效）
/// - 逼和 → draw 音效
///
/// 注意：victory/defeat 仅在玩家参与的对局中调用方额外播放，此函数不处理
export function playMoveSounds(opts: {
  uci: string;
  oldFen: string;
  inCheck: boolean;
  gameOver: boolean;
  status: string; // "playing" | "checkmate" | "stalemate"
}): void {
  const { uci, oldFen, inCheck, gameOver, status } = opts;

  if (gameOver) {
    if (status === "checkmate") {
      playSound("checkmate");
    } else if (status === "stalemate") {
      playSound("draw");
    }
    return;
  }

  // 非终局：播放走子音 + 可能的将军音
  const moveType = classifyMove(uci);
  const isCapture = isCaptureMove(oldFen, uci.slice(2, 4));

  if (moveType === "castle") {
    playSound("castle");
  } else if (moveType === "promote") {
    playSound("promote");
  } else if (isCapture) {
    playSound("capture");
  } else {
    playSound("move");
  }

  if (inCheck) {
    // 延迟 150ms 播放将军音效，避免与走子音效并发冲突导致其中一个被吞
    setTimeout(() => playSound("check"), 150);
  }
}
