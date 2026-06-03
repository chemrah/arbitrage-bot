import { useEffect } from 'react';
import { WalletConnector } from '../components/WalletConnector';
import { Mempool3DVisualizer } from '../components/Mempool3DVisualizer';
import { LiquidityBubbleMap } from '../components/LiquidityBubbleMap';
import { ArbDashboard } from '../components/ArbDashboard';
import { useAppStore } from '../lib/store';

function generateMockData() {
  const store = useAppStore.getState();

  const mockOpportunities = [
    {
      id: '0x1a2b...',
      type: 'Triangular V3' as const,
      pools: ['0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640', '0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8', '0xcbcdf9626bc03e24f779434178a73a0b4bad62ed'],
      tokenIn: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      tokenOut: '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48',
      amountIn: '1000000000000000000',
      estimatedProfit: '25000000000000000',
      successProbability: 0.82,
      timestamp: Date.now(),
      route: [],
      status: 'Ready',
    },
    {
      id: '0x3c4d...',
      type: 'CEX-DEX' as const,
      pools: ['0x7a250d5630b4cf539739df2c5dacb4c659f2488d'],
      tokenIn: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      tokenOut: '0x514910771AF9Ca656af840dff83E8264EcF986CA',
      amountIn: '50000000000000000000',
      estimatedProfit: '120000000000000000',
      successProbability: 0.65,
      timestamp: Date.now(),
      route: [],
      status: 'Simulating',
    },
    {
      id: '0x5e6f...',
      type: 'Cross-Pool V3/V4' as const,
      pools: ['0x11b815efb8f581194ae79006d24e0d814b7697f6'],
      tokenIn: '0xdAC17F958D2ee523a2206206994597C13D831ec7',
      tokenOut: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      amountIn: '100000000000000000000',
      estimatedProfit: '300000000000000000',
      successProbability: 0.45,
      timestamp: Date.now(),
      route: [],
      status: 'Scanning',
    },
    {
      id: '0x7g8h...',
      type: 'JIT Liquidity' as const,
      pools: ['0x4e68Ccd3E89f51C3074ca5072bbAC773960DdFa9'],
      tokenIn: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      tokenOut: '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984',
      amountIn: '75000000000000000000',
      estimatedProfit: '80000000000000000',
      successProbability: 0.55,
      timestamp: Date.now(),
      route: [],
      status: 'Scanning',
    },
    {
      id: '0x9i0j...',
      type: 'Triangular V3' as const,
      pools: ['0x5777d92f208679db4b9778590fa3cab3ac9e2168'],
      tokenIn: '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599',
      tokenOut: '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
      amountIn: '100000000',
      estimatedProfit: '45000000000000000',
      successProbability: 0.91,
      timestamp: Date.now(),
      route: [],
      status: 'Ready',
    },
  ];

  mockOpportunities.forEach((op) => store.addOpportunity(op as any));
}

export default function Home() {
  useEffect(() => {
    generateMockData();
  }, []);

  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Navbar */}
      <nav
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 24px',
          borderBottom: '1px solid var(--border-color)',
          background: 'var(--bg-secondary)',
        }}
      >
        <div className="flex items-center gap-3">
          <div
            style={{
              width: 28,
              height: 28,
              borderRadius: 6,
              background: 'linear-gradient(135deg, #448aff, #7c4dff)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: 14,
              fontWeight: 700,
            }}
          >
            A
          </div>
          <span className="font-bold text-sm">Arbitrage Engine</span>
          <span className="text-[10px] text-[var(--text-secondary)] px-2 py-0.5 rounded bg-[var(--bg-hover)]">
            v0.1.0
          </span>
        </div>
        <WalletConnector />
      </nav>

      {/* Main Content */}
      <div style={{ flex: 1, overflow: 'auto' }}>
        {/* 3D Visualization Row */}
        <div className="grid grid-cols-2 gap-4" style={{ padding: '16px 24px 0' }}>
          <Mempool3DVisualizer />
          <LiquidityBubbleMap />
        </div>

        {/* Dashboard */}
        <ArbDashboard />
      </div>

      {/* Status Bar */}
      <footer
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 24px',
          borderTop: '1px solid var(--border-color)',
          background: 'var(--bg-secondary)',
          fontSize: 10,
          color: 'var(--text-secondary)',
        }}
      >
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-green)]" />
            Engine Online
          </span>
          <span>Block: 21,048,392</span>
          <span>Base: 12.4 gwei</span>
        </div>
        <div className="flex items-center gap-4">
          <span>WS: Connected</span>
          <span>Flashbots: Relay Active</span>
          <span>Revm: Local Fork</span>
        </div>
      </footer>
    </div>
  );
}
