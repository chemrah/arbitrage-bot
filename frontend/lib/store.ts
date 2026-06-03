import { create } from 'zustand';
import {
  WalletState,
  ArbOpportunity,
  MempoolTransaction,
  TelemetryData,
  BribeConfig,
  SlippageConfig,
  PoolBubble,
} from './types';

interface AppStore {
  wallet: WalletState;
  setWallet: (wallet: WalletState) => void;

  opportunities: ArbOpportunity[];
  setOpportunities: (ops: ArbOpportunity[]) => void;
  addOpportunity: (op: ArbOpportunity) => void;
  updateOpportunity: (id: string, updates: Partial<ArbOpportunity>) => void;

  mempoolTxns: MempoolTransaction[];
  setMempoolTxns: (txns: MempoolTransaction[]) => void;
  addMempoolTxn: (txn: MempoolTransaction) => void;

  pools: PoolBubble[];
  setPools: (pools: PoolBubble[]) => void;

  telemetry: TelemetryData;
  setTelemetry: (t: TelemetryData) => void;

  bribeConfig: BribeConfig;
  setBribeConfig: (config: BribeConfig) => void;

  slippageConfig: SlippageConfig;
  setSlippageConfig: (config: SlippageConfig) => void;

  autoPilot: boolean;
  setAutoPilot: (on: boolean) => void;

  isSimulating: boolean;
  setIsSimulating: (v: boolean) => void;

  terminalOutput: string[];
  addTerminalOutput: (line: string) => void;
  clearTerminal: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  wallet: {
    address: null,
    isConnected: false,
    chainId: null,
    balance: '0',
  },
  setWallet: (wallet) => set({ wallet }),

  opportunities: [],
  setOpportunities: (opportunities) => set({ opportunities }),
  addOpportunity: (op) =>
    set((state) => ({
      opportunities: [op, ...state.opportunities].slice(0, 100),
    })),
  updateOpportunity: (id, updates) =>
    set((state) => ({
      opportunities: state.opportunities.map((op) =>
        op.id === id ? { ...op, ...updates } : op
      ),
    })),

  mempoolTxns: [],
  setMempoolTxns: (mempoolTxns) => set({ mempoolTxns }),
  addMempoolTxn: (txn) =>
    set((state) => ({
      mempoolTxns: [txn, ...state.mempoolTxns].slice(0, 500),
    })),

  pools: [],
  setPools: (pools) => set({ pools }),

  telemetry: {
    eventCapture: 0,
    mathComputation: 0,
    evmSimulation: 0,
    bundleSubmission: 0,
    totalLatency: 0,
  },
  setTelemetry: (telemetry) => set({ telemetry }),

  bribeConfig: { percent: 35, enabled: true },
  setBribeConfig: (bribeConfig) => set({ bribeConfig }),

  slippageConfig: { tolerance: 0.5 },
  setSlippageConfig: (slippageConfig) => set({ slippageConfig }),

  autoPilot: false,
  setAutoPilot: (autoPilot) => set({ autoPilot }),

  isSimulating: false,
  setIsSimulating: (isSimulating) => set({ isSimulating }),

  terminalOutput: ['arb-engine v0.1.0 initialized', '> scanning for opportunities...'],
  addTerminalOutput: (line) =>
    set((state) => ({
      terminalOutput: [...state.terminalOutput, `> ${line}`].slice(-200),
    })),
  clearTerminal: () => set({ terminalOutput: ['terminal cleared'] }),
}));
