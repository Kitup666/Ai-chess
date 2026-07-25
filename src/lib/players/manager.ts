/// 对局编排器：驱动当前轮到方走棋，处理自对弈循环、步间延迟、停止控制
///
/// 使用方式：
///   const manager = new PlayerManager(whiteType, blackType, sfOpts);
///   await manager.driveTurn(state, async (result) => { updateGameState(result.state); });
///   manager.stop(); // 中断自对弈
import type { GameStateDto, PlayerType, MoveResult } from "../types";
import type { Player } from "./types";
import { createHumanPlayer } from "./human";
import { createStockfishPlayer } from "./stockfish";
import type { StockfishOptions } from "./stockfish";
import { createDeepSeekPlayer } from "./deepseek";

export type { StockfishOptions };

export class PlayerManager {
  private whitePlayer: Player;
  private blackPlayer: Player;
  private stopped = false;
  private stepCount = 0;
  private readonly maxSteps = 200;
  private readonly stepDelay = 300;
  /// 代际标记：每次 driveTurn/reset 递增，旧循环检测到代际不匹配立即退出
  /// 用于防止暂停/重置后旧循环的 requestMove Promise resolve 继续走棋导致双重走棋
  private generation = 0;

  constructor(white: PlayerType, black: PlayerType, sfOpts: StockfishOptions) {
    this.whitePlayer = createPlayer(white, sfOpts);
    this.blackPlayer = createPlayer(black, sfOpts);
  }

  /// 驱动当前轮到方走棋（自动主体连续走，直到 Human 方或游戏结束或停止）
  /// @param state 当前局面
  /// @param onMoveApplied 每步走法应用后的回调（更新 UI 状态 + 动画），支持 async
  async driveTurn(
    state: GameStateDto,
    onMoveApplied: (result: MoveResult) => Promise<void> | void
  ): Promise<void> {
    // 代际递增：让上一轮正在 await requestMove 的旧循环退出
    this.generation++;
    this.stopped = false;
    this.stepCount = 0;
    const myGen = this.generation;
    await this.driveLoop(state, onMoveApplied, myGen);
  }

  /// Human 走棋后继续驱动（若对方是自动主体）
  async continueAfterHumanMove(
    state: GameStateDto,
    onMoveApplied: (result: MoveResult) => Promise<void> | void
  ): Promise<void> {
    // Human 走棋触发的驱动沿用当前代际（不递增，因为是同一对局的延续）
    const myGen = this.generation;
    await this.driveLoop(state, onMoveApplied, myGen);
  }

  private async driveLoop(
    state: GameStateDto,
    onMoveApplied: (result: MoveResult) => Promise<void> | void,
    myGen: number
  ): Promise<void> {
    let current = state;
    while (!this.stopped && myGen === this.generation) {
      if (current.status !== "playing") break;
      if (this.stepCount >= this.maxSteps) break;
      const player = current.turn === "white" ? this.whitePlayer : this.blackPlayer;
      // Human 方：停止循环，等待点击事件触发 continueAfterHumanMove
      if (!player.isAutomatic || !player.requestMove) break;
      // 自动主体请求走法
      const result = await player.requestMove(current);
      // 代际检查：等待期间若被 reset/driveTurn 取消，丢弃结果直接退出
      if (this.stopped || myGen !== this.generation) return;
      await onMoveApplied(result);
      // 应用走法后再次检查代际（防止 onMoveApplied 期间发生 reset）
      if (this.stopped || myGen !== this.generation) return;
      this.stepCount++;
      if (result.game_over) break;
      current = result.state;
      // 步间延迟（自对弈时便于观察，不卡顿）
      await new Promise((r) => setTimeout(r, this.stepDelay));
    }
  }

  /// 停止自对弈循环
  stop(): void {
    this.stopped = true;
  }

  /// 重置停止状态和步数计数
  /// 代际递增：让旧循环退出，避免暂停后 Promise resolve 继续走棋
  reset(): void {
    this.generation++;
    this.stopped = false;
    this.stepCount = 0;
  }
}

function createPlayer(type: PlayerType, sfOpts: StockfishOptions): Player {
  switch (type) {
    case "human":
      return createHumanPlayer();
    case "stockfish":
      return createStockfishPlayer(sfOpts);
    case "deepseek":
      return createDeepSeekPlayer();
  }
}
