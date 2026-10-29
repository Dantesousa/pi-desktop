export type Part =
  | { ci: number; kind: "text"; text: string }
  | { ci: number; kind: "thinking"; text: string }
  | {
      ci: number;
      kind: "tool";
      id: string;
      name: string;
      args: any;
      output?: string;
      isError?: boolean;
      durationMs?: number;
      running: boolean;
    };

export type Chip = {
  name: string;
  kind: "image" | "text" | "binary";
  mime: string;
  size: number;
  preview?: string;
  data?: string;
  text?: string;
  stagedPath?: string;
};

export type Msg = {
  key: string;
  role: "user" | "assistant" | "system";
  parts: Part[];
  ts: number;
  streaming?: boolean;
  error?: string;
  model?: string;
  chips?: Chip[];
  note?: string;
};

export type ModelInfo = {
  id: string;
  name: string;
  provider: string;
  reasoning?: boolean;
  input?: string[];
  contextWindow?: number;
  maxTokens?: number;
};

export type MemoryEntry = {
  index: number;
  raw: string;
  category: string | null;
  body: string;
  created: string | null;
  last: string | null;
  failure_reason: string | null;
};

export type MemoryFile = {
  target: string;
  path: string;
  entries: MemoryEntry[];
  chars: number;
  limit: number;
};

export type MemoryStats = {
  memoryChars: number;
  memoryLimit: number;
  userChars: number;
  userLimit: number;
  failureChars: number;
  failureLimit: number;
  dbEntries: number;
  dbSessions: number;
  dbMessages: number;
  dbBytes: number;
};

export type SessionInfo = {
  id: string;
  path: string;
  cwd: string;
  name: string | null;
  startedAt: string;
  modifiedMs: number;
  size: number;
  messages: number;
};

export type SkillInfo = { name: string; description: string; path: string; scope: string };

export type SessionHit = {
  role: string;
  content: string;
  timestamp: string;
  project: string;
  sessionId: string;
};

export type DbMemory = {
  id: number;
  target: string;
  project: string | null;
  category: string | null;
  content: string;
  failureReason: string | null;
  created: string;
  lastReferenced: string;
  hitCount: number;
};
