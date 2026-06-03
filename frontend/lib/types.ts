export interface PoolData {
  address: string;
  token0: string;
  token1: string;
  fee: number;
  liquidity: string;
  volume24h: number;
  tvl: number;
  sqrtPriceX96: string;
  tick: number;
  volatility: number;
}

export interface MempoolTransaction {
  hash: string;
  from: string;
  to: string;
  value: string;
  gasPrice: string;
  gasLimit: string;
  input: string;
  timestamp: number;
  poolAddress?: string;
  swapDirection?: 'buy' | 'sell';
  swapValue?: number;
}

export interface ArbOpportunity {
  id: string;
  type: ArbType;
  pools: string[];
  tokenIn: string;
  tokenOut: string;
  amountIn: string;
  estimatedProfit: string;
  successProbability: number;
  timestamp: number;
  route: SwapStep[];
  status: OpportunityStatus;
}

export enum ArbType {
  TriangularV3 = 'Triangular V3',
  CrossPoolV3V4 = 'Cross-Pool V3/V4',
  CexDex = 'CEX-DEX',
  JitLiquidity = 'JIT Liquidity',
}

export enum OpportunityStatus {
  Scanning = 'Scanning',
  Simulating = 'Simulating',
  Ready = 'Ready',
  Executing = 'Executing',
  Success = 'Success',
  Failed = 'Failed',
}

export interface SwapStep {
  pool: string;
  zeroForOne: boolean;
  amount: string;
  sqrtPriceLimit: string;
}

export interface TelemetryData {
  eventCapture: number;
  mathComputation: number;
  evmSimulation: number;
  bundleSubmission: number;
  totalLatency: number;
}

export interface PoolBubble {
  address: string;
  name: string;
  liquidity: number;
  volume: number;
  volatility: number;
  x: number;
  y: number;
  r: number;
}

export interface SimulationResult {
  profitable: boolean;
  profit: string;
  gasUsed: number;
  tipAmount: string;
}

export interface WalletState {
  address: string | null;
  isConnected: boolean;
  chainId: number | null;
  balance: string;
}

export interface BribeConfig {
  percent: number;
  enabled: boolean;
}

export interface SlippageConfig {
  tolerance: number;
}

export type StrategyCommand = {
  raw: string;
  parsed: {
    action: 'arbitrage' | 'monitor' | 'cancel';
    tokenIn?: string;
    tokenOut?: string;
    chain?: string;
    minSpread?: number;
    amount?: string;
  } | null;
};
