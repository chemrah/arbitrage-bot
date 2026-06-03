import { useMemo } from 'react';
import { useAppStore } from '../lib/store';
import { PoolBubble } from '../lib/types';

function Bubble({
  bubble,
  onHover,
}: {
  bubble: PoolBubble;
  onHover: (b: PoolBubble | null) => void;
}) {
  const volatilityColor = useMemo(() => {
    if (bubble.volatility > 0.7) return 'rgba(255, 23, 68, 0.6)';
    if (bubble.volatility > 0.4) return 'rgba(255, 215, 64, 0.6)';
    return 'rgba(0, 230, 118, 0.6)';
  }, [bubble.volatility]);

  const borderColor = useMemo(() => {
    if (bubble.volatility > 0.7) return '#ff1744';
    if (bubble.volatility > 0.4) return '#ffd740';
    return '#00e676';
  }, [bubble.volatility]);

  return (
    <g
      transform={`translate(${bubble.x}, ${bubble.y})`}
      onMouseEnter={() => onHover(bubble)}
      onMouseLeave={() => onHover(null)}
      style={{ cursor: 'pointer' }}
    >
      <circle
        r={bubble.r}
        fill={volatilityColor}
        stroke={borderColor}
        strokeWidth={1.5}
        opacity={0.8}
      />
      <text
        textAnchor="middle"
        dy="0.3em"
        fill="white"
        fontSize={Math.max(8, bubble.r * 0.3)}
        fontWeight={600}
        style={{ pointerEvents: 'none' }}
      >
        {bubble.name.length > 8 ? bubble.name.slice(0, 7) + '..' : bubble.name}
      </text>
    </g>
  );
}

export function LiquidityBubbleMap() {
  const pools = useAppStore((s) => s.pools);

  const positionedPools = useMemo(() => {
    if (pools.length === 0) {
      return generateMockPools();
    }
    return pools;
  }, [pools]);

  const handleHover = (bubble: PoolBubble | null) => {
    const el = document.getElementById('bubble-tooltip');
    if (el && bubble) {
      el.textContent = `${bubble.name} | Liq: ${(bubble.liquidity / 1e6).toFixed(1)}M | Vol: ${bubble.volatility.toFixed(2)}`;
      el.style.display = 'block';
    } else if (el) {
      el.style.display = 'none';
    }
  };

  return (
    <div className="card" style={{ height: 400 }}>
      <div className="card-header">
        <span className="card-title">Liquidity Bubble Map</span>
        <div className="flex gap-3 text-[10px]">
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-[var(--accent-green)]" />
            Low Vol
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-[var(--accent-yellow)]" />
            Med Vol
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-[var(--accent-red)]" />
            High Vol
          </span>
        </div>
      </div>
      <div style={{ position: 'relative', height: 340, width: '100%' }}>
        <svg width="100%" height="100%" viewBox="0 0 800 340">
          {positionedPools.map((bubble, i) => (
            <Bubble key={i} bubble={bubble} onHover={handleHover} />
          ))}
        </svg>
        <div
          id="bubble-tooltip"
          style={{
            display: 'none',
            position: 'absolute',
            bottom: 10,
            left: 10,
            background: 'rgba(0,0,0,0.8)',
            padding: '6px 12px',
            borderRadius: 4,
            fontSize: 11,
            color: '#e8e8f0',
            border: '1px solid #2a2a3a',
            zIndex: 10,
          }}
        />
      </div>
    </div>
  );
}

function generateMockPools(): PoolBubble[] {
  const pairs = [
    { name: 'WETH/USDC', liquidity: 450_000_000, volatility: 0.35 },
    { name: 'WETH/USDT', liquidity: 380_000_000, volatility: 0.30 },
    { name: 'WBTC/WETH', liquidity: 290_000_000, volatility: 0.55 },
    { name: 'LINK/WETH', liquidity: 120_000_000, volatility: 0.65 },
    { name: 'UNI/WETH', liquidity: 85_000_000, volatility: 0.70 },
    { name: 'AAVE/WETH', liquidity: 65_000_000, volatility: 0.60 },
    { name: 'CRV/WETH', liquidity: 45_000_000, volatility: 0.50 },
    { name: 'MKR/WETH', liquidity: 35_000_000, volatility: 0.45 },
    { name: 'SNX/WETH', liquidity: 25_000_000, volatility: 0.75 },
    { name: 'COMP/WETH', liquidity: 30_000_000, volatility: 0.55 },
  ];

  const maxLiq = Math.max(...pairs.map((p) => p.liquidity));
  const cols = 5;
  const rows = Math.ceil(pairs.length / cols);
  const cellW = 800 / cols;
  const cellH = 340 / rows;

  return pairs.map((pair, i) => {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const r = 10 + (pair.liquidity / maxLiq) * 40;
    return {
      address: `0x${i.toString(16).padStart(40, '0')}`,
      name: pair.name,
      liquidity: pair.liquidity,
      volume: pair.liquidity * 0.3,
      volatility: pair.volatility,
      x: col * cellW + cellW / 2 + (Math.random() - 0.5) * 30,
      y: row * cellH + cellH / 2 + (Math.random() - 0.5) * 20,
      r,
    };
  });
}
