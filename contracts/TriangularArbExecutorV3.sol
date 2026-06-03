// SPDX-License-Identifier: BUSL-1.1
pragma solidity >=0.8.24;

import "./FlashTipping.sol";

interface IUniswapV3Pool {
    function swap(
        address recipient,
        bool zeroForOne,
        int256 amountSpecified,
        uint160 sqrtPriceLimitX96,
        bytes calldata data
    ) external returns (int256 amount0, int256 amount1);
    function token0() external view returns (address);
    function token1() external view returns (address);
    function fee() external view returns (uint24);
    function liquidity() external view returns (uint128);
    function slot0()
        external
        view
        returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked);
}

interface IWETH9 {
    function deposit() external payable;
    function withdraw(uint256) external;
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
}

interface IERC20 {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
    function approve(address, uint256) external returns (bool);
}

contract TriangularArbExecutorV3 is FlashTipping {
    struct SwapStep {
        address pool;
        bool zeroForOne;
        uint160 sqrtPriceLimitX96;
    }

    struct Route {
        SwapStep[3] steps;
        address inputToken;
        address intermediateToken;
        address outputToken;
    }

    IWETH9 public immutable weth;
    uint256 private constant MIN_PROFIT_BPS = 5;

    uint256 private _locked;
    uint256 private _gasSnapshot;
    uint256 private _initialWETHBal;
    uint256 private _initialInputBal;

    constructor(address _owner, address _executor, uint256 _bribeBps, address _weth)
        FlashTipping(_owner, _executor, _bribeBps)
    {
        require(_weth != address(0), "TA: weth zero");
        weth = IWETH9(_weth);
    }

    function executeTriangularArb(Route calldata route, uint256 amountIn)
        external
        onlyExecutor
        returns (int256 profit)
    {
        require(_locked == 0, "TA: reentrant");
        _locked = 1;

        uint256 initialGas = gasleft();
        _initialInputBal = IERC20(route.inputToken).balanceOf(address(this));

        IUniswapV3Pool(route.steps[0].pool).swap(
            address(this),
            route.steps[0].zeroForOne,
            int256(amountIn),
            route.steps[0].sqrtPriceLimitX96,
            abi.encode(route)
        );

        uint256 finalBal = IERC20(route.inputToken).balanceOf(address(this));
        uint256 gasSpent = (initialGas - gasleft()) * tx.gasprice;

        require(finalBal > _initialInputBal, "TA: no profit");
        uint256 grossProfit = finalBal - _initialInputBal;
        require(grossProfit > gasSpent + (grossProfit * MIN_PROFIT_BPS / BPS_DIVISOR), "TA: below min profit");

        (uint256 tip, uint256 netProfit) = _calculateTip(grossProfit, gasSpent);
        uint256 executorReward = netProfit;

        if (route.inputToken != address(weth) && tip > 0) {
            _swapToWeth(route.inputToken, tip + netProfit);
            weth.withdraw(weth.balanceOf(address(this)));
            _sendTip(tip);
        } else if (tip > 0) {
            _sendTip(tip);
        }

        _locked = 0;
        emit ArbitrageExecuted(
            keccak256(abi.encodePacked(route.steps[0].pool, route.steps[1].pool, route.steps[2].pool)),
            route.inputToken,
            int256(grossProfit),
            tip,
            gasSpent
        );
    }

    function uniswapV3SwapCallback(
        int256 amount0Delta,
        int256 amount1Delta,
        bytes calldata data
    ) external {
        Route memory route = abi.decode(data, (Route));

        require(
            msg.sender == route.steps[0].pool ||
            msg.sender == route.steps[1].pool ||
            msg.sender == route.steps[2].pool,
            "TA: invalid callback"
        );

        if (amount0Delta > 0) {
            IERC20(IUniswapV3Pool(msg.sender).token0()).transfer(
                msg.sender, uint256(amount0Delta)
            );
        } else if (amount1Delta > 0) {
            IERC20(IUniswapV3Pool(msg.sender).token1()).transfer(
                msg.sender, uint256(amount1Delta)
            );
        }

        if (msg.sender == route.steps[0].pool) {
            _executeStep(route, 1);
        } else if (msg.sender == route.steps[1].pool) {
            _executeStep(route, 2);
        }
    }

    function _executeStep(Route memory route, uint256 stepIndex) internal {
        address tokenIn = stepIndex == 1 ? route.intermediateToken : route.outputToken;
        uint256 balance = IERC20(tokenIn).balanceOf(address(this));
        if (balance == 0) return;
        IUniswapV3Pool(route.steps[stepIndex].pool).swap(
            address(this),
            route.steps[stepIndex].zeroForOne,
            int256(balance),
            route.steps[stepIndex].sqrtPriceLimitX96,
            abi.encode(route)
        );
    }

    function _swapToWeth(address token, uint256 amount) internal {
        if (token == address(weth)) return;
        uint256 bal = IERC20(token).balanceOf(address(this));
        if (bal < amount) amount = bal;
        if (amount == 0) return;
        IERC20(token).transfer(address(weth), amount);
        IWETH9(address(weth)).withdraw(amount);
    }

    function rescueTokens(address token, uint256 amount) external onlyOwner {
        if (token == address(0)) {
            (bool sent,) = payable(owner).call{value: amount}("");
            require(sent, "TA: rescue ETH failed");
        } else {
            IERC20(token).transfer(owner, amount);
        }
    }
}
