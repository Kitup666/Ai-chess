/// Stockfish 引擎封装（前端 WASM，通过 Web Worker 通信）
///
/// 基于 stockfish.js 18 (lite single-threaded)，通过 UCI 协议与引擎交互。
/// 引擎在 Worker 中运行，不阻塞 UI 主线程。
///
/// 使用方式：
///   const engine = new StockfishEngine();
///   await engine.load();
///   engine.onInfo = (info) => { ... };
///   engine.search(fen, { depth: 15 });
///   engine.onBestMove = (best) => { ... };
///
/// 或一次性求走法：
///   const best = await engine.getBestMove(fen, { movetime: 1000 });

/// 引擎状态
export type EngineStatus = "unloaded" | "loading" | "ready" | "searching";

/// 搜索选项（UCI go 命令参数）
export interface GoOptions {
  /// 搜索深度（plies），如 15
  depth?: number;
  /// 搜索时间（毫秒），如 1000
  movetime?: number;
  /// 无限搜索（需调用 stop() 停止）
  infinite?: boolean;
  /// 搜索节点数上限
  nodes?: number;
}

/// 搜索信息（UCI info 行解析结果）
export interface SearchInfo {
  /// 搜索深度
  depth: number;
  /// 选择性深度
  seldepth?: number;
  /// 分数（cp=厘兵值，mate=将杀步数）
  score?: { type: "cp" | "mate"; value: number };
  /// MultiPV 序号（1=最佳，2=次佳...）
  multipv?: number;
  /// 已搜索节点数
  nodes?: number;
  /// 每秒节点数
  nps?: number;
  /// 已搜索时间（毫秒）
  time?: number;
  /// 主要变着（UCI 走法数组）
  pv?: string[];
}

/// 最佳走法结果（UCI bestmove 行解析）
export interface BestMove {
  /// 最佳走法（UCI，如 "e2e4"）
  move: string;
  /// 思考走法（对手可能的回复，如 "e7e5"）
  ponder?: string;
}

/// Stockfish 引擎封装类
///
/// 管理 Worker 生命周期、UCI 通信、难度设置、搜索控制。
/// 一个实例对应一个 Worker，销毁时调用 terminate()。
export class StockfishEngine {
  private worker: Worker | null = null;
  private _status: EngineStatus = "unloaded";
  private loadResolve: (() => void) | null = null;
  private loadReject: ((e: Error) => void) | null = null;
  private readyResolve: (() => void) | null = null;
  private bestMoveResolve: ((best: BestMove) => void) | null = null;
  private bestMoveReject: ((e: Error) => void) | null = null;

  /// 回调：状态变化
  onStatusChange?: (status: EngineStatus) => void;
  /// 回调：搜索信息更新（info 行，流式推送）
  onInfo?: (info: SearchInfo) => void;
  /// 回调：搜索完成（bestmove 行）
  onBestMove?: (best: BestMove) => void;

  /// 引擎版本（从 uciok 前的 id 行解析）
  version: string = "";

  get status(): EngineStatus {
    return this._status;
  }

  private set status(s: EngineStatus) {
    this._status = s;
    this.onStatusChange?.(s);
  }

  /// 加载引擎（创建 Worker 并初始化 UCI）
  ///
  /// 加载流程：创建 Worker → 发送 uci → 收到 uciok → 发送 isready → 收到 readyok
  /// 首次加载可能需要 1-2 秒（解析 7MB WASM）
  async load(): Promise<void> {
    if (this.worker) {
      if (this.status === "ready" || this.status === "searching") return;
      throw new Error("引擎正在加载中");
    }

    this.status = "loading";

    // Worker URL：不带 hash 后缀
    // stockfish.js 检测 hash 含 "worker" 时认为是子 Worker（不初始化）；
    // 主 Worker 不带 hash，stockfish.js 自动从 location.pathname 推导 wasm 路径
    // （/stockfish/stockfish.js → /stockfish/stockfish.wasm）
    const base = import.meta.env.BASE_URL || "/";
    const workerUrl = `${base}stockfish/stockfish.js`;

    return new Promise((resolve, reject) => {
      this.loadResolve = resolve;
      this.loadReject = reject;

      try {
        const worker = new Worker(workerUrl);
        this.worker = worker;
        worker.onmessage = (e: MessageEvent) => this.handleMessage(e.data);
        worker.onerror = (e: ErrorEvent) => {
          this.status = "unloaded";
          this.loadReject?.(new Error(`引擎加载失败: ${e.message}`));
          this.loadReject = null;
          this.loadResolve = null;
        };
        // 发送 uci 命令，等待 uciok
        worker.postMessage("uci");
      } catch (e) {
        this.status = "unloaded";
        reject(e instanceof Error ? e : new Error(String(e)));
        this.loadResolve = null;
        this.loadReject = null;
      }
    });
  }

  /// 处理引擎输出消息（UCI 协议行）
  private handleMessage(line: string): void {
    line = String(line).trim();
    if (!line) return;

    // 解析 UCI 命令响应
    if (line.startsWith("id name ")) {
      this.version = line.slice(8).trim();
      return;
    }
    if (line === "uciok") {
      // UCI 初始化完成，发送 isready
      this.worker?.postMessage("isready");
      return;
    }
    if (line === "readyok") {
      // 引擎就绪
      if (this.status === "loading") {
        this.status = "ready";
        this.loadResolve?.();
        this.loadResolve = null;
        this.loadReject = null;
      }
      this.readyResolve?.();
      this.readyResolve = null;
      return;
    }
    if (line.startsWith("info ")) {
      const info = this.parseInfoLine(line);
      if (info) this.onInfo?.(info);
      return;
    }
    if (line.startsWith("bestmove")) {
      const best = this.parseBestMove(line);
      this.status = "ready";
      this.onBestMove?.(best);
      this.bestMoveResolve?.(best);
      this.bestMoveResolve = null;
      this.bestMoveReject = null;
      return;
    }
    // 其他输出（如 option 行）忽略
  }

  /// 解析 UCI info 行
  /// 格式：info depth 20 seldepth 25 multipv 1 score cp 35 nodes 1234567 nps 234567 time 5267 pv e2e4 e7e5 g1f3 ...
  private parseInfoLine(line: string): SearchInfo | null {
    try {
      const tokens = line.split(/\s+/);
      const info: SearchInfo = { depth: 0 };
      let i = 1; // 跳过 "info"
      while (i < tokens.length) {
        const key = tokens[i];
        const val = tokens[i + 1];
        switch (key) {
          case "depth":
            info.depth = parseInt(val, 10) || 0;
            i += 2;
            break;
          case "seldepth":
            info.seldepth = parseInt(val, 10) || 0;
            i += 2;
            break;
          case "multipv":
            info.multipv = parseInt(val, 10) || 1;
            i += 2;
            break;
          case "nodes":
            info.nodes = parseInt(val, 10) || 0;
            i += 2;
            break;
          case "nps":
            info.nps = parseInt(val, 10) || 0;
            i += 2;
            break;
          case "time":
            info.time = parseInt(val, 10) || 0;
            i += 2;
            break;
          case "score": {
            // score cp 35 或 score mate 5
            const type = tokens[i + 1] as "cp" | "mate";
            const value = parseInt(tokens[i + 2], 10) || 0;
            info.score = { type, value };
            i += 3;
            break;
          }
          case "pv": {
            // pv 后面所有 token 都是走法
            info.pv = tokens.slice(i + 1);
            i = tokens.length; // 结束循环
            break;
          }
          default:
            i += 1;
        }
      }
      return info.depth > 0 ? info : null;
    } catch {
      return null;
    }
  }

  /// 解析 UCI bestmove 行
  /// 格式：bestmove e2e4 ponder e7e5
  private parseBestMove(line: string): BestMove {
    const tokens = line.split(/\s+/);
    const move = tokens[1] || "";
    let ponder: string | undefined;
    if (tokens[2] === "ponder" && tokens[3]) {
      ponder = tokens[3];
    }
    return { move, ponder };
  }

  /// 等待引擎就绪（发送 isready，等 readyok）
  /// 用于设置选项后确保引擎处理完毕
  private async waitReady(): Promise<void> {
    if (!this.worker) throw new Error("引擎未加载");
    return new Promise((resolve) => {
      this.readyResolve = resolve;
      this.worker!.postMessage("isready");
    });
  }

  /// 设置难度等级（0-20）
  ///
  /// Skill Level 0=最弱，20=最强（默认20）。
  /// 低于10会引入随机走法，适合人类对战。
  async setSkillLevel(level: number): Promise<void> {
    if (!this.worker) throw new Error("引擎未加载");
    const clamped = Math.max(0, Math.min(20, Math.floor(level)));
    this.worker.postMessage(`setoption name Skill Level value ${clamped}`);
    await this.waitReady();
  }

  /// 限制强度为指定 Elo（约 1320-3190）
  ///
  /// 需先设置 UCI_LimitStrength=true，再设 UCI_Elo。
  /// 仅 Stockfish 12+ 支持。
  async setElo(elo: number): Promise<void> {
    if (!this.worker) throw new Error("引擎未加载");
    const clamped = Math.max(1320, Math.min(3190, Math.floor(elo)));
    this.worker.postMessage("setoption name UCI_LimitStrength value true");
    this.worker.postMessage(`setoption name UCI_Elo value ${clamped}`);
    await this.waitReady();
  }

  /// 设置 MultiPV（输出多条主要变着）
  ///
  /// n=1 只输出最佳，n=3 输出前3条。用于分析模式。
  async setMultiPV(n: number): Promise<void> {
    if (!this.worker) throw new Error("引擎未加载");
    const clamped = Math.max(1, Math.min(20, Math.floor(n)));
    this.worker.postMessage(`setoption name MultiPV value ${clamped}`);
    await this.waitReady();
  }

  /// 设置 Hash 大小（MB）
  async setHash(mb: number): Promise<void> {
    if (!this.worker) throw new Error("引擎未加载");
    const clamped = Math.max(16, Math.min(2048, Math.floor(mb)));
    this.worker.postMessage(`setoption name Hash value ${clamped}`);
    await this.waitReady();
  }

  /// 新游戏（清空 Hash）
  async newGame(): Promise<void> {
    if (!this.worker) throw new Error("引擎未加载");
    this.worker.postMessage("ucinewgame");
    await this.waitReady();
  }

  /// 开始搜索（异步，结果通过 onInfo/onBestMove 回调返回）
  ///
  /// @param fen 当前局面 FEN
  /// @param opts 搜索选项（depth/movetime/infinite/nodes）
  search(fen: string, opts: GoOptions = {}): void {
    if (!this.worker) throw new Error("引擎未加载");
    if (this.status === "searching") this.stop();
    this.status = "searching";
    // 设置局面
    this.worker.postMessage(`position fen ${fen}`);
    // 构建 go 命令
    let go = "go";
    if (opts.depth) go += ` depth ${opts.depth}`;
    else if (opts.movetime) go += ` movetime ${opts.movetime}`;
    else if (opts.nodes) go += ` nodes ${opts.nodes}`;
    else if (opts.infinite) go += " infinite";
    else go += ` depth 15`; // 默认深度15
    this.worker.postMessage(go);
  }

  /// 停止搜索
  stop(): void {
    if (!this.worker) return;
    this.worker.postMessage("stop");
  }

  /// 一次性求最佳走法（Promise 模式）
  ///
  /// 内部调用 search 并等待 bestmove。搜索期间 onInfo 仍会触发。
  async getBestMove(fen: string, opts: GoOptions = {}): Promise<BestMove> {
    return new Promise((resolve, reject) => {
      this.bestMoveResolve = resolve;
      this.bestMoveReject = reject;
      this.search(fen, opts);
    });
  }

  /// 销毁引擎（终止 Worker）
  terminate(): void {
    if (this.worker) {
      try {
        this.worker.postMessage("quit");
      } catch {}
      this.worker.terminate();
      this.worker = null;
    }
    this.status = "unloaded";
    this.bestMoveReject?.(new Error("引擎已销毁"));
    this.bestMoveReject = null;
    this.bestMoveResolve = null;
    this.loadReject?.(new Error("引擎已销毁"));
    this.loadReject = null;
    this.loadResolve = null;
    this.readyResolve = null;
  }
}
