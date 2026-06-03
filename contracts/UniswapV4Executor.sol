// SPDX-License-Identifier: BUSL-1.1
pragma solidity >=0.8.24;

import "./FlashTipping.sol";

type Currency is address;

struct PoolKey {
    Currency currency0;
    Currency currency1;
    uint24 fee;
    int24 tickSpacing;
    address hooks;
}

interface IPoolManager {
    struct SwapParams {
        bool zeroForOne;
        int256 amountSpecified;
        uint160 sqrtPriceLimitX96;
    }

    function unlock(bytes calldata data) external returns (bytes memory result);
    function swap(
        PoolKey memory key,
        SwapParams memory params,
        bytes calldata data
    ) external returns (int256 delta0, int256 delta1);
    function settle(Currency currency) external payable;
    function take(Currency currency, address to, uint256 amount) external;
    function mint(
        PoolKey memory key,
        int24 tickLower,
        int24 tickUpper,
        uint256 amount,
        bytes calldata data
    ) external returns (uint256 liquidity, uint256 amount0, uint256 amount1);
    function burn(
        PoolKey memory key,
        int24 tickLower,
        int24 tickUpper,
        uint256 liquidity,
        bytes calldata data
    ) external returns (uint256 amount0, uint256 amount1);
}

interface IERC20 {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
}

contract UniswapV4Executor is FlashTipping {
    IPoolManager public immutable poolManager;

    uint256 private _locked;
    int256 private _expectedDelta0;
    int256 private _expectedDelta1;
    Currency private _inputCurrency;
    Currency private _outputCurrency;

    bytes32 internal constant _TLOAD_ACCOUNTING_SLOT = keccak256("v4.accounting");
    bytes32 internal constant _TLOAD_PROFIT_SLOT = keccak256("v4.profit");

    constructor(
        address _owner,
        address _executor,
        uint256 _bribeBps,
        address _poolManager
    ) FlashTipping(_owner, _executor, _bribeBps) {
        require(_poolManager != address(0), "V4: poolManager zero");
        poolManager = IPoolManager(_poolManager);
    }

    function executeV4Arb(
        PoolKey[] calldata route,
        Currency inputCurrency,
        Currency outputCurrency,
        uint256 amountIn
    ) external onlyExecutor returns (int256 profit) {
        require(_locked == 0, "V4: reentrant");
        _locked = 1;
        _inputCurrency = inputCurrency;
        _outputCurrency = outputCurrency;

        uint256 initialGas = gasleft();
        uint256 initialBal = _balanceOf(inputCurrency);

        poolManager.unlock(abi.encode(route, amountIn, inputCurrency, outputCurrency));

        uint256 finalBal = _balanceOf(inputCurrency);
        uint256 gasSpent = (initialGas - gasleft()) * tx.gasprice;

        require(finalBal > initialBal, "V4: no profit");
        uint256 grossProfit = finalBal - initialBal;
        require(grossProfit > gasSpent, "V4: non-profitable");

        (uint256 tip, uint256 netProfit) = _calculateTip(grossProfit, gasSpent);

        if (tip > 0 && address(this).balance >= tip) {
            _sendTip(tip);
        }

        _locked = 0;
        emit ArbitrageExecuted(
            keccak256(abi.encodePacked(route.length)),
            Currency.unwrap(inputCurrency),
            int256(grossProfit),
            tip,
            gasSpent
        );
    }

    function lockAcquired(bytes calldata data)
        external
        returns (bytes memory)
    {
        require(msg.sender == address(poolManager), "V4: not poolManager");

        (PoolKey[] memory route, uint256 amountIn, Currency inputCurrency, Currency outputCurrency) =
            abi.decode(data, (PoolKey[], uint256, Currency, Currency));

        uint256 len = route.length;
        Currency currentCurrency = inputCurrency;
        int256 cumulativeDelta;

        for (uint256 i = 0; i < len; i++) {
            bool zeroForOne = currentCurrency == route[i].currency0;

            IPoolManager.SwapParams memory params = IPoolManager.SwapParams({
                zeroForOne: zeroForOne,
                amountSpecified: i == 0 ? int256(amountIn) : int256(cumulativeDelta > 0 ? cumulativeDelta : -cumulativeDelta),
                sqrtPriceLimitX96: zeroForOne ? 4295128740 : 146144670348521010328727385220117881058519721300779033241323110371833446839200
            });

            (int256 delta0, int256 delta1) = poolManager.swap(route[i], params, "");

            if (delta0 < 0) {
                cumulativeDelta = delta0;
                poolManager.take(route[i].currency0, address(this), uint256(-delta0));
                currentCurrency = route[i].currency0;
            }
            if (delta1 < 0) {
                cumulativeDelta = delta1;
                poolManager.take(route[i].currency1, address(this), uint256(-delta1));
                currentCurrency = route[i].currency1;
            }
        }

        for (uint256 i = 0; i < len; i++) {
            uint256 bal0 = _balanceOf(route[i].currency0);
            uint256 bal1 = _balanceOf(route[i].currency1);

            if (bal0 > 0) {
                IERC20(Currency.unwrap(route[i].currency0)).transfer(
                    address(poolManager), bal0
                );
                poolManager.settle(route[i].currency0);
            }
            if (bal1 > 0) {
                IERC20(Currency.unwrap(route[i].currency1)).transfer(
                    address(poolManager), bal1
                );
                poolManager.settle(route[i].currency1);
            }
        }

        return abi.encode(0);
    }

    function _balanceOf(Currency currency) internal view returns (uint256) {
        (bool success, bytes memory data) = Currency.unwrap(currency).staticcall(
            abi.encodeWithSelector(IERC20.balanceOf.selector, address(this))
        );
        require(success, "V4: balanceOf failed");
        return abi.decode(data, (uint256));
    }

    function rescueTokens(address token, uint256 amount) external onlyOwner {
        if (token == address(0)) {
            (bool sent,) = payable(owner).call{value: amount}("");
            require(sent, "V4: rescue failed");
        } else {
            IERC20(token).transfer(owner, amount);
        }
    }
}
