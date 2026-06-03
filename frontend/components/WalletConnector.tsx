import { useEffect, useCallback } from 'react';
import { useAppStore } from '../lib/store';

export function WalletConnector() {
  const { wallet, setWallet } = useAppStore();

  const connectWallet = useCallback(async () => {
    if (typeof window === 'undefined' || !(window as any).ethereum) {
      useAppStore.getState().addTerminalOutput('MetaMask not detected');
      return;
    }

    try {
      const accounts: string[] = await (window as any).ethereum.request({
        method: 'eth_requestAccounts',
      });
      const chainId: string = await (window as any).ethereum.request({
        method: 'eth_chainId',
      });
      const balance: string = await (window as any).ethereum.request({
        method: 'eth_getBalance',
        params: [accounts[0], 'latest'],
      });

      setWallet({
        address: accounts[0],
        isConnected: true,
        chainId: parseInt(chainId, 16),
        balance: (BigInt(balance) / BigInt(10 ** 15)).toString(),
      });

      useAppStore.getState().addTerminalOutput(
        `wallet connected: ${accounts[0].slice(0, 6)}...${accounts[0].slice(-4)}`
      );
    } catch (e: any) {
      useAppStore.getState().addTerminalOutput(`connection failed: ${e.message}`);
    }
  }, [setWallet]);

  const disconnectWallet = useCallback(() => {
    setWallet({
      address: null,
      isConnected: false,
      chainId: null,
      balance: '0',
    });
    useAppStore.getState().addTerminalOutput('wallet disconnected');
  }, [setWallet]);

  useEffect(() => {
    if (typeof window !== 'undefined' && (window as any).ethereum) {
      const handleAccountsChanged = (accounts: string[]) => {
        if (accounts.length === 0) {
          disconnectWallet();
        } else {
          setWallet({ ...wallet, address: accounts[0] });
        }
      };
      (window as any).ethereum.on('accountsChanged', handleAccountsChanged);
      return () => {
        (window as any).ethereum?.removeListener?.('accountsChanged', handleAccountsChanged);
      };
    }
  }, [wallet, setWallet, disconnectWallet]);

  return (
    <div className="flex items-center gap-3">
      {wallet.isConnected ? (
        <>
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-[var(--accent-green)]" />
            <span className="text-xs text-[var(--text-secondary)]">
              {wallet.address?.slice(0, 6)}...{wallet.address?.slice(-4)}
            </span>
            <span className="text-xs text-[var(--text-secondary)] px-1 py-0.5 rounded bg-[var(--bg-hover)]">
              {wallet.balance} ETH
            </span>
          </div>
          <button
            onClick={disconnectWallet}
            className="btn btn-outline text-xs px-3 py-1"
          >
            Disconnect
          </button>
        </>
      ) : (
        <button onClick={connectWallet} className="btn btn-primary text-xs px-4 py-1.5">
          Connect Wallet
        </button>
      )}
    </div>
  );
}
