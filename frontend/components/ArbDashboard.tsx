import { useMemo, useCallback, useState } from 'react';
import {
  LineChart, Line, AreaChart, Area, BarChart, Bar,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  Cell,
} from 'recharts';
import { Zap, TrendingUp, Activity, Shield, Cpu, Gauge } from 'lucide-react';
import { useAppStore } from '../lib/store';
import { ArbType, OpportunityStatus } from '../lib/types';

function MiniMetric({ icon: Icon, label, value, color }: {
  icon: any; label: string; value: string; color?: string;
}) {
  return (
    <div className="card flex-1 min-w-[120px]">
      <div className="flex items-center gap-2 mb-2">
        <Icon size={14} color={color || 'var(--accent-blue)'} />
        <span className="metric-label">{label}</span>
      </div>
      <div className="metric-value" style={{ color: color || 'var(--text-primary)' }}>
        {value}
      </div>
    </div>
  );
}

function ArbScanTable() {
  const opportunities = useAppStore((s) => s.opportunities);

  return (
    <div className="card flex-1 min-h-[300px]">
      <div className="card-header">
        <span className="card-title">Arbitrage Scan Results</span>
        <span className="text-[10px] text-[var(--text-secondary)]">
          {opportunities.length} opportunities
        </span>
      </div>
      <div style={{ overflow: 'auto', maxHeight: 260 }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11 }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border-color)', color: 'var(--text-secondary)' }}>
              <th style={{ padding: '6px 8px', textAlign: 'left' }}>Type</th>
              <th style={{ padding: '6px 8px', textAlign: 'left' }}>Route</th>
              <th style={{ padding: '6px 8px', textAlign: 'right' }}>Profit</th>
              <th style={{ padding: '6px 8px', textAlign: 'right' }}>Prob</th>
              <th style={{ padding: '6px 8px', textAlign: 'right' }}>Status</th>
              <th style={{ padding: '6px 8px', textAlign: 'center' }}>Action</th>
            </tr>
          </thead>
          <tbody>
            {opportunities.length === 0 ? (
              <tr>
                <td colSpan={6} style={{ padding: 20, textAlign: 'center', color: 'var(--text-secondary)' }}>
                  No opportunities found. Waiting for scanner...
                </td>
              </tr>
            ) : (
              opportunities.slice(0, 15).map((op) => (
                <tr
                  key={op.id}
                  style={{ borderBottom: '1px solid var(--border-color)', transition: 'background 0.15s' }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-hover)')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <td style={{ padding: '6px 8px' }}>
                    <span className={`badge ${
                      op.type === ArbType.TriangularV3 ? 'badge-blue' :
                      op.type === ArbType.CexDex ? 'badge-yellow' :
                      op.type === ArbType.JitLiquidity ? 'badge-purple' :
                      'badge-green'
                    }`}>
                      {op.type}
                    </span>
                  </td>
                  <td style={{ padding: '6px 8px', fontFamily: 'monospace', fontSize: 10 }}>
                    {op.pools.slice(0, 3).map((p) => p.slice(0, 6)).join(' → ')}
                  </td>
                  <td style={{ padding: '6px 8px', textAlign: 'right', color: 'var(--accent-green)', fontWeight: 600 }}>
                    ${(BigInt(op.estimatedProfit) / BigInt(10 ** 15)).toString()}
                  </td>
                  <td style={{ padding: '6px 8px', textAlign: 'right' }}>
                    <span className={op.successProbability > 0.7 ? 'text-[var(--accent-green)]' : 'text-[var(--accent-yellow)]'}>
                      {(op.successProbability * 100).toFixed(0)}%
                    </span>
                  </td>
                  <td style={{ padding: '6px 8px', textAlign: 'right' }}>
                    <span className={`badge ${
                      op.status === OpportunityStatus.Ready ? 'badge-green' :
                      op.status === OpportunityStatus.Simulating ? 'badge-yellow' :
                      op.status === OpportunityStatus.Success ? 'badge-blue' :
                      op.status === OpportunityStatus.Failed ? 'badge-red' :
                      'badge-blue'
                    }`}>
                      {op.status}
                    </span>
                  </td>
                  <td style={{ padding: '6px 8px', textAlign: 'center' }}>
                    <button
                      className="btn btn-primary text-[10px] px-2 py-1"
                      onClick={() => useAppStore.getState().addTerminalOutput(
                        `executing ${op.id}...`
                      )}
                    >
                      Execute
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function BribeSlider() {
  const { bribeConfig, setBribeConfig } = useAppStore();

  return (
    <div className="card">
      <div className="card-header">
        <span className="card-title">Bribe / Tip Optimizer</span>
        <span className="text-xs font-mono">{bribeConfig.percent}%</span>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-[10px] text-[var(--text-secondary)]">Greed</span>
        <input
          type="range"
          min={0}
          max={95}
          value={bribeConfig.percent}
          onChange={(e) => setBribeConfig({ ...bribeConfig, percent: parseInt(e.target.value) })}
          className="slider flex-1"
        />
        <span className="text-[10px] text-[var(--text-secondary)]">Success</span>
      </div>
      <div className="flex justify-between mt-2 text-[9px] text-[var(--text-secondary)]">
        <span>Low tip = low inclusion</span>
        <span>High tip = max MEV extraction</span>
      </div>
    </div>
  );
}

function SlippageHeatmap() {
  const data = useMemo(() => {
    const rows = [];
    for (let slippage = 0.1; slippage <= 2.0; slippage += 0.3) {
      const row: any = { slippage: `${slippage.toFixed(1)}%` };
      for (let blockCost = 1; blockCost <= 5; blockCost++) {
        const inclusionChance = Math.min(95, Math.max(5,
          20 + slippage * 15 - blockCost * 5 + Math.random() * 10
        ));
        const sandwichRisk = Math.min(90, Math.max(5,
          slippage * 35 - blockCost * 3 + Math.random() * 10
        ));
        row[`b${blockCost}`] = {
          inclusion: inclusionChance,
          sandwich: sandwichRisk,
          score: inclusionChance - sandwichRisk * 0.5,
        };
      }
      rows.push(row);
    }
    return rows;
  }, []);

  const [hovered, setHovered] = useState<{ slip: string; block: string; data: any } | null>(null);

  return (
    <div className="card">
      <div className="card-header">
        <span className="card-title">Slippage Heatmap Simulator</span>
      </div>
      <div style={{ overflow: 'auto' }}>
        <table style={{ fontSize: 10, borderCollapse: 'collapse', width: '100%' }}>
          <thead>
            <tr>
              <th style={{ padding: '4px 6px', textAlign: 'left', color: 'var(--text-secondary)' }}>Slippage</th>
              {[1, 2, 3, 4, 5].map((b) => (
                <th key={b} style={{ padding: '4px 6px', textAlign: 'center', color: 'var(--text-secondary)' }}>
                  B{b}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.map((row) => (
              <tr key={row.slippage}>
                <td style={{ padding: '4px 6px', color: 'var(--text-secondary)' }}>{row.slippage}</td>
                {[1, 2, 3, 4, 5].map((b) => {
                  const d = row[`b${b}`];
                  const score = d.score;
                  const bg = score > 60 ? 'rgba(0,230,118,0.3)' :
                    score > 40 ? 'rgba(255,215,64,0.3)' :
                    'rgba(255,23,68,0.3)';
                  return (
                    <td
                      key={b}
                      style={{
                        padding: '4px 6px',
                        textAlign: 'center',
                        background: hovered?.slip === row.slippage && hovered?.block === `b${b}` ? 'rgba(68,138,255,0.2)' : bg,
                        cursor: 'pointer',
                        borderRadius: 2,
                      }}
                      onMouseEnter={() => setHovered({ slip: row.slippage, block: `b${b}`, data: d })}
                      onMouseLeave={() => setHovered(null)}
                    >
                      <div>{(d.score).toFixed(0)}</div>
                      <div style={{ fontSize: 8, color: 'var(--text-secondary)' }}>
                        S:{(d.sandwich).toFixed(0)}%
                      </div>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {hovered && (
        <div className="mt-2 text-[10px] flex gap-4 text-[var(--text-secondary)]">
          <span>Score: {hovered.data.score.toFixed(1)}</span>
          <span>Inclusion: {hovered.data.inclusion.toFixed(1)}%</span>
          <span>Sandwich Risk: {hovered.data.sandwich.toFixed(1)}%</span>
        </div>
      )}
    </div>
  );
}

function TelemetryConsole() {
  const telemetry = useAppStore((s) => s.telemetry);
  const autoPilot = useAppStore((s) => s.autoPilot);
  const setAutoPilot = useAppStore((s) => s.setAutoPilot);
  const terminalOutput = useAppStore((s) => s.terminalOutput);
  const addTerminalOutput = useAppStore((s) => s.addTerminalOutput);
  const clearTerminal = useAppStore((s) => s.clearTerminal);

  const [cmdInput, setCmdInput] = useState('');

  const handleCommand = useCallback((cmd: string) => {
    const trimmed = cmd.trim().toLowerCase();
    addTerminalOutput(`$ ${cmd}`);

    if (trimmed === 'help') {
      addTerminalOutput('Available commands: status, start, stop, config, bribe <0-95>, clear, help');
    } else if (trimmed === 'status') {
      addTerminalOutput(`Auto-pilot: ${autoPilot ? 'ON' : 'OFF'} | Bribe: ${useAppStore.getState().bribeConfig.percent}% | Opportunities: ${useAppStore.getState().opportunities.length}`);
    } else if (trimmed === 'start') {
      setAutoPilot(true);
      addTerminalOutput('Auto-pilot engaged. Scanning & executing autonomously.');
    } else if (trimmed === 'stop') {
      setAutoPilot(false);
      addTerminalOutput('Auto-pilot disengaged. Manual mode.');
    } else if (trimmed === 'clear') {
      clearTerminal();
    } else if (trimmed.startsWith('bribe ')) {
      const val = parseInt(trimmed.split(' ')[1]);
      if (!isNaN(val) && val >= 0 && val <= 95) {
        useAppStore.getState().setBribeConfig({ percent: val, enabled: true });
        addTerminalOutput(`Bribe set to ${val}%`);
      } else {
        addTerminalOutput('Invalid bribe value (0-95)');
      }
    } else if (trimmed.startsWith('arbitrage') || trimmed.includes('arb')) {
      addTerminalOutput('Parsing strategy: arbitrage WETH/USDC if spread > 0.3%');
      addTerminalOutput('Strategy configured. Scanner activated.');
    } else {
      addTerminalOutput(`Unknown command: ${cmd}`);
    }
  }, [autoPilot, setAutoPilot, addTerminalOutput, clearTerminal]);

  const telemetryItems = [
    { label: 'Event Capture', value: telemetry.eventCapture, unit: 'μs', max: 500 },
    { label: 'Math Computation', value: telemetry.mathComputation, unit: 'μs', max: 300 },
    { label: 'EVM Simulation', value: telemetry.evmSimulation, unit: 'μs', max: 800 },
    { label: 'Bundle Submission', value: telemetry.bundleSubmission, unit: 'μs', max: 2000 },
  ];

  return (
    <div className="card">
      <div className="card-header">
        <span className="card-title">Microsecond Telemetry & Terminal</span>
        <div className="flex items-center gap-3">
          <div className="text-[10px] text-[var(--text-secondary)]">
            Total: {telemetry.totalLatency}μs
          </div>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2 mb-3">
        {telemetryItems.map((item) => (
          <div key={item.label} className="flex items-center gap-2">
            <span className="text-[10px] text-[var(--text-secondary)] w-24">{item.label}</span>
            <div className="flex-1 h-1.5 bg-[var(--bg-secondary)] rounded-full overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.min(100, (item.value / item.max) * 100)}%`,
                  background: item.value < item.max * 0.5
                    ? 'var(--accent-green)'
                    : item.value < item.max * 0.8
                      ? 'var(--accent-yellow)'
                      : 'var(--accent-red)',
                }}
              />
            </div>
            <span className="text-[10px] font-mono w-16 text-right">
              {item.value}{item.unit}
            </span>
          </div>
        ))}
      </div>
      <div
        className="terminal"
        style={{
          background: '#000',
          borderRadius: 4,
          padding: 8,
          height: 120,
          overflow: 'auto',
          fontFamily: 'monospace',
          fontSize: 11,
          lineHeight: 1.6,
        }}
      >
        {terminalOutput.slice(-12).map((line, i) => (
          <div key={i} style={{ color: line.startsWith('> $') ? '#00e676' : '#8888a0' }}>
            {line}
          </div>
        ))}
      </div>
      <div className="flex gap-2 mt-2">
        <input
          type="text"
          value={cmdInput}
          onChange={(e) => setCmdInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && cmdInput.trim()) {
              handleCommand(cmdInput.trim());
              setCmdInput('');
            }
          }}
          placeholder="Enter command or natural language strategy..."
          style={{
            flex: 1,
            background: '#000',
            border: '1px solid var(--border-color)',
            borderRadius: 4,
            padding: '6px 8px',
            color: '#e8e8f0',
            fontSize: 11,
            fontFamily: 'monospace',
            outline: 'none',
          }}
        />
        <button
          onClick={() => {
            if (cmdInput.trim()) {
              handleCommand(cmdInput.trim());
              setCmdInput('');
            }
          }}
          className="btn btn-primary text-[10px]"
        >
          Send
        </button>
      </div>
    </div>
  );
}

function ProfitChart() {
  const data = useMemo(() => {
    const now = Date.now();
    return Array.from({ length: 30 }, (_, i) => ({
      time: new Date(now - (30 - i) * 1000).toLocaleTimeString(),
      profit: Math.random() * 0.5 + 0.1,
      cumulative: Array.from({ length: i + 1 }, () => Math.random() * 0.3 + 0.05).reduce((a, b) => a + b, 0),
    }));
  }, []);

  return (
    <div className="card flex-1 min-h-[200px]">
      <div className="card-header">
        <span className="card-title">Profit History</span>
      </div>
      <ResponsiveContainer width="100%" height={160}>
        <AreaChart data={data}>
          <defs>
            <linearGradient id="profitGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#00e676" stopOpacity={0.3} />
              <stop offset="95%" stopColor="#00e676" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#1a1a2e" />
          <XAxis dataKey="time" tick={{ fontSize: 9, fill: '#8888a0' }} />
          <YAxis tick={{ fontSize: 9, fill: '#8888a0' }} />
          <Tooltip
            contentStyle={{
              background: '#1a1a24',
              border: '1px solid #2a2a3a',
              borderRadius: 4,
              fontSize: 11,
            }}
          />
          <Area type="monotone" dataKey="profit" stroke="#00e676" fill="url(#profitGradient)" />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

export function ArbDashboard() {
  const opportunities = useAppStore((s) => s.opportunities);
  const autoPilot = useAppStore((s) => s.autoPilot);
  const setAutoPilot = useAppStore((s) => s.setAutoPilot);

  const stats = useMemo(() => {
    const profitable = opportunities.filter(
      (o) => o.status === OpportunityStatus.Success
    ).length;
    const failed = opportunities.filter(
      (o) => o.status === OpportunityStatus.Failed
    ).length;
    return {
      total: opportunities.length,
      profitable,
      failed,
      successRate: opportunities.length > 0
        ? ((profitable / opportunities.length) * 100).toFixed(1)
        : '0.0',
    };
  }, [opportunities]);

  return (
    <div className="space-y-4" style={{ padding: '16px 24px' }}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold tracking-tight">Arbitrage Engine</h1>
          <p className="text-[10px] text-[var(--text-secondary)] mt-0.5">
            Uniswap V3/V4 · CEX-DEX · JIT Liquidity · Flash Mint
          </p>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-[var(--text-secondary)]">Auto-Pilot</span>
            <div
              className={`toggle ${autoPilot ? 'active' : ''}`}
              onClick={() => setAutoPilot(!autoPilot)}
            />
          </div>
        </div>
      </div>

      {/* Mini Metrics */}
      <div className="flex gap-3 flex-wrap">
        <MiniMetric icon={Zap} label="Opportunities" value={stats.total.toString()} color="var(--accent-blue)" />
        <MiniMetric icon={TrendingUp} label="Profitable" value={stats.profitable.toString()} color="var(--accent-green)" />
        <MiniMetric icon={Activity} label="Success Rate" value={`${stats.successRate}%`} color="var(--accent-purple)" />
        <MiniMetric icon={Shield} label="Failed" value={stats.failed.toString()} color="var(--accent-red)" />
        <MiniMetric icon={Cpu} label="Latency (p50)" value="142μs" color="var(--accent-yellow)" />
        <MiniMetric icon={Gauge} label="Gas Target" value="<110k" />
      </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-3 gap-4">
        <div className="col-span-2">
          <ArbScanTable />
        </div>
        <div className="col-span-1 space-y-4">
          <BribeSlider />
          <SlippageHeatmap />
        </div>
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-2 gap-4">
        <ProfitChart />
        <TelemetryConsole />
      </div>
    </div>
  );
}
