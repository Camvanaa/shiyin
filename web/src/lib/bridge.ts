// 主窗口与子窗口（桌面歌词、迷你播放器）之间的通信协议。
// 播放只在主窗口进行；子窗口只显示状态、发送控制指令。

export interface BridgeState {
  track: { id: number; name: string; artist: string; cover: string } | null;
  playing: boolean;
  /** 秒。子窗口用 at + (now - sentAt) 推算当前进度 */
  at: number;
  sentAt: number;
  duration: number;
  /** 当前和下一行歌词 */
  line: string;
  trans: string;
  next: string;
  /** 当前行开始和结束的时间（秒），用于逐行进度动画 */
  lineStart: number;
  lineEnd: number;
  /** 封面主色 "r g b" */
  tint: string;
}

export type BridgeControl =
  | { type: 'toggle' }
  | { type: 'next' }
  | { type: 'prev' }
  | { type: 'seek'; value: number }
  | { type: 'show-main' }
  | { type: 'hello' };

export const EV_STATE = 'player:state';
export const EV_CONTROL = 'player:control';
