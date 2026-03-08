declare module 'ws' {
  export interface MessageLike {
    toString(): string;
  }

  export interface WebSocketConnection {
    on(event: 'message', listener: (raw: MessageLike) => void): void;
    send(data: string): void;
  }

  export class WebSocketServer {
    constructor(options: { port: number });
    address(): { port: number } | string | null;
    on(event: 'connection', listener: (socket: WebSocketConnection) => void): void;
    close(callback: (error?: Error) => void): void;
  }
}
