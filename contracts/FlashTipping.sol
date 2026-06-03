// SPDX-License-Identifier: BUSL-1.1
pragma solidity >=0.8.24;

abstract contract FlashTipping {
    address public owner;
    address public executor;
    uint256 public bribeBps;
    uint256 public constant BPS_DIVISOR = 10_000;
    uint256 public constant MAX_BRIBE_BPS = 9_500;

    event BribeUpdated(uint256 indexed oldBps, uint256 indexed newBps);
    event ExecutorUpdated(address indexed oldExecutor, address indexed newExecutor);
    event ArbitrageExecuted(
        bytes32 indexed poolId,
        address indexed tokenIn,
        int256 profit,
        uint256 tip,
        uint256 gasSpent
    );
    event TipSent(address indexed coinbase, uint256 amount);

    modifier onlyExecutor() {
        require(msg.sender == executor, "FT: not executor");
        _;
    }

    modifier onlyOwner() {
        require(msg.sender == owner, "FT: not owner");
        _;
    }

    constructor(address _owner, address _executor, uint256 _bribeBps) {
        require(_owner != address(0), "FT: owner zero");
        require(_executor != address(0), "FT: executor zero");
        require(_bribeBps <= MAX_BRIBE_BPS, "FT: bribe too high");
        owner = _owner;
        executor = _executor;
        bribeBps = _bribeBps;
    }

    function setBribeBps(uint256 _bribeBps) external onlyOwner {
        require(_bribeBps <= MAX_BRIBE_BPS, "FT: bribe too high");
        uint256 old = bribeBps;
        bribeBps = _bribeBps;
        emit BribeUpdated(old, _bribeBps);
    }

    function setExecutor(address _executor) external onlyOwner {
        require(_executor != address(0), "FT: executor zero");
        address old = executor;
        executor = _executor;
        emit ExecutorUpdated(old, _executor);
    }

    function _calculateTip(uint256 profit, uint256 gasSpent)
        internal
        view
        returns (uint256 tip, uint256 netProfit)
    {
        if (profit <= gasSpent) return (0, 0);
        unchecked {
            uint256 gross = profit - gasSpent;
            tip = (gross * bribeBps) / BPS_DIVISOR;
            netProfit = gross - tip;
        }
    }

    function _sendTip(uint256 tipAmount) internal {
        if (tipAmount == 0) return;
        uint256 bal = address(this).balance;
        if (bal < tipAmount) return;
        (bool sent,) = payable(block.coinbase).call{value: tipAmount, gas: 2300}("");
        if (sent) emit TipSent(block.coinbase, tipAmount);
    }

    function _verifyProfit(uint256 initial, uint256 finalBal, uint256 gasCost)
        internal
        pure
    {
        require(finalBal > initial + gasCost, "FT: non-profitable");
    }

    receive() external payable virtual {}
}
