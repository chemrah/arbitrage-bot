# Arbitrage Bot — Codespace Setup Guide

## Prerequisites

- GitHub account with access to [Codespaces](https://github.com/codespaces)
- An Ethereum RPC provider (Infura, Alchemy, or local node)
- A funded wallet private key on the target network
- (Optional) Flashbots relay access

---

## Step 1: Launch Codespace

1. Go to `https://github.com/chemrah/arbitrage-bot`
2. Click the green **Code** button → **Codespaces** tab → **Create codespace on main**
3. Wait 30–60 seconds for the environment to provision

---

## Step 2: Install System Dependencies

Run in the Codespace terminal:

```bash
# Update packages
sudo apt-get update -y

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version   # should be >= 1.85

# Install Node.js 22+ (via nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.4/install.sh | bash
source "$HOME/.bashrc"
nvm install 22
nvm use 22
node --version   # should be >= 22

# Install Foundry (for Solidity compilation)
curl -L https://foundry.paradigm.xyz | bash
source "$HOME/.bashrc"
foundryup
forge --version

# Install pnpm
npm install -g pnpm

# Install binaryen (for wasm-opt if needed)
sudo apt-get install -y binaryen
```

---

## Step 3: Build the Rust Engine

```bash
cd rust-engine

# Build (release with optimizations)
cargo build --release

# Expected output: target/release/arb-engine.exe
```

> **Note:** First build takes 5–10 minutes due to dependency compilation.

---

## Step 4: Install Frontend Dependencies

```bash
cd frontend
pnpm install
```

---

## Step 5: Configure Environment

Create `rust-engine/.env`:

```bash
# Required
RUST_LOG=info
WS_RPC=wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
HTTP_RPC=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
EXECUTOR_ADDRESS=0xYourExecutorContractAddress
EXECUTOR_PRIVATE_KEY=0xYourPrivateKey

# Optional — MEV relays
FLASHBOTS_ENDPOINT=https://relay.flashbots.net
BEAVER_BUILD_ENDPOINT=https://rpc.beaverbuild.org
TITAN_ENDPOINT=https://rpc.titanbuilder.xyz
BUILDER069_ENDPOINT=https://rpc.builder069.xyz

# Optional — CEX hedging
BINANCE_API_KEY=your_binance_key
BINANCE_SECRET=your_binance_secret
OKX_API_KEY=your_okx_key
OKX_SECRET=your_okx_secret
OKX_PASSPHRASE=your_okx_passphrase

# Bot settings
BRIBE_PERCENT=35
AUTO_PILOT=false
```

Create `frontend/.env.local`:

```bash
NEXT_PUBLIC_WS_RPC=wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
NEXT_PUBLIC_HTTP_RPC=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
```

---

## Step 6: Deploy Smart Contracts

```bash
# Install Foundry if not done above
cd contracts

# Compile
forge build

# Deploy FlashTipping parent contract (deploy args: owner, executor, bribeBps)
forge create --rpc-url $HTTP_RPC \
  --private-key $EXECUTOR_PRIVATE_KEY \
  contracts/FlashTipping.sol:FlashTipping \
  --constructor-args 0xYourOwner 0xYourExecutor 3500

# Deploy TriangularArbExecutorV3 (deploy args: owner, executor, bribeBps, weth)
forge create --rpc-url $HTTP_RPC \
  --private-key $EXECUTOR_PRIVATE_KEY \
  contracts/TriangularArbExecutorV3.sol:TriangularArbExecutorV3 \
  --constructor-args 0xYourOwner 0xYourExecutor 3500 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2

# Deploy UniswapV4Executor (deploy args: owner, executor, bribeBps, poolManager)
forge create --rpc-url $HTTP_RPC \
  --private-key $EXECUTOR_PRIVATE_KEY \
  contracts/UniswapV4Executor.sol:UniswapV4Executor \
  --constructor-args 0xYourOwner 0xYourExecutor 3500 0x000000000004444c5dc75cB358380D2e3dE5A5E7

# Deploy MakerDAOMintWrapper (deploy args: owner, executor, bribeBps, flashMinter, daiJoin)
forge create --rpc-url $HTTP_RPC \
  --private-key $EXECUTOR_PRIVATE_KEY \
  contracts/MakerDAOMintWrapper.sol:MakerDAOMintWrapper \
  --constructor-args 0xYourOwner 0xYourExecutor 3500 0x1EB4CF3A948E7D72A198fe073cCb8C7a948cD853 0x0A59649758aa4d66E25f08Dd01271e891fe52199
```

> Replace `0xYourOwner`, `0xYourExecutor`, and RPC URLs with your actual values.

---

## Step 7: Run the Engine

### Start the Rust Engine

```bash
cd rust-engine
cargo run --release -- \
  --ws-rpc "$WS_RPC" \
  --http-rpc "$HTTP_RPC" \
  --executor-address "$EXECUTOR_ADDRESS" \
  --executor-private-key "$EXECUTOR_PRIVATE_KEY" \
  --bribe-percent 35 \
  --auto-pilot false
```

Or use the `.env` file with `dotenv`:

```bash
source .env
cargo run --release
```

### Start the Frontend (separate terminal)

```bash
cd frontend
pnpm dev
```

Open the Codespace Forwarded Ports dialog — the frontend will be at `http://localhost:3000`.

---

## Step 8: Usage

### Terminal Commands (in the frontend dashboard terminal)

| Command | Action |
|---|---|
| `help` | List available commands |
| `status` | Show engine status |
| `start` | Enable auto-pilot (autonomous trading) |
| `stop` | Disable auto-pilot |
| `bribe 50` | Set bribe percentage to 50% |
| `clear` | Clear terminal |
| `arbitrage WETH/USDC if spread > 0.3%` | NLP strategy command |

### Bribe / Tip Slider

Controls the percentage of profit sent to block builders (0% = greedy, 95% = max inclusion chance).

### Slippage Heatmap

Hover over cells to see inclusion probability vs sandwich attack risk for each slippage/block-cost combination.

---

## Common Issues

| Problem | Fix |
|---|---|
| `Connection refused` on WS RPC | Ensure your RPC provider supports WebSocket connections |
| `insufficient funds` | Fund your executor wallet with ETH for gas |
| `forge: command not found` | Run `foundryup` or reinstall Foundry |
| Rust build takes too long | First build is slow — subsequent builds use cache |
| Frontend shows no data | Engine must be running and connected to a WebSocket RPC |

---

## File Reference

```
arbitrage-bot/
├── contracts/
│   ├── FlashTipping.sol             # Abstract base (bribe, profit check, tip)
│   ├── TriangularArbExecutorV3.sol  # V3 flash swap executor
│   ├── UniswapV4Executor.sol        # V4 unlock/lock executor
│   └── MakerDAOMintWrapper.sol      # DAI flash mint wrapper
├── rust-engine/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                  # Entry point, task orchestration
│       ├── listener.rs              # WebSocket mempool listener
│       ├── math.rs                  # Uniswap V3 tick/math (off-chain)
│       ├── simulator.rs             # Revm local EVM simulation
│       ├── bundler.rs               # Flashbots bundle builder
│       ├── cex_hedger.rs            # Binance/OKX price streams
│       └── solver.rs                # CoW Swap / UniswapX intent solver
├── frontend/
│   ├── package.json
│   ├── pages/
│   │   ├── index.tsx                # Main dashboard layout
│   │   └── _app.tsx                 # Next.js app wrapper
│   ├── components/
│   │   ├── Mempool3DVisualizer.tsx  # Three.js particle system
│   │   ├── LiquidityBubbleMap.tsx   # SVG bubble chart
│   │   ├── ArbDashboard.tsx         # Scan table, charts, terminal
│   │   └── WalletConnector.tsx      # MetaMask connect
│   ├── lib/
│   │   ├── types.ts                 # TypeScript interfaces
│   │   └── store.ts                 # Zustand state
│   └── styles/
│       └── globals.css              # Tailwind v4 + custom styles
└── SETUP.md                         # This file
```

---

## Security Notes

- **Never commit `.env` files** — they're in `.gitignore`
- Revoke your GitHub token after use
- The executor wallet should only hold gas funds (no idle token balance)
- Test on a testnet fork before going live: `anvil --fork-url $HTTP_RPC`
