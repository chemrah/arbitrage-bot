// SPDX-License-Identifier: BUSL-1.1
pragma solidity >=0.8.24;

import "./FlashTipping.sol";

interface IDaiJoin {
    function dai() external view returns (address);
    function vat() external view returns (address);
    function join(address, uint256) external payable;
    function exit(address, uint256) external;
}

interface IVat {
    function hope(address) external;
    function move(address, address, uint256) external;
    function flux(bytes32, address, address, uint256) external;
    function can(address, address) external view returns (uint256);
    function dai(address) external view returns (uint256);
    function sin(address) external view returns (uint256);
    function ilks(bytes32) external view returns (uint256, uint256, uint256, uint256, uint256);
    function urns(bytes32, address) external view returns (uint256, uint256);
    function frob(bytes32, address, address, address, int256, int256) external;
    function fold(bytes32, address, int256) external;
}

interface IDai {
    function balanceOf(address) external view returns (uint256);
    function transfer(address, uint256) external returns (bool);
}

interface IDssFlash {
    function dai() external view returns (address);
    function maxFlashLoan(address) external view returns (uint256);
    function flashFee(address, uint256) external view returns (uint256);
    function flashLoan(
        IERC3156FlashBorrower receiver,
        address token,
        uint256 amount,
        bytes calldata data
    ) external returns (bool);
}

interface IERC3156FlashBorrower {
    function onFlashLoan(
        address initiator,
        address token,
        uint256 amount,
        uint256 fee,
        bytes calldata data
    ) external returns (bytes32);
}

contract MakerDAOMintWrapper is FlashTipping, IERC3156FlashBorrower {
    IDssFlash public immutable flashMinter;
    IDai public immutable dai;
    address public immutable vat;
    bytes32 public constant ILK = bytes32("ETH-A");

    uint256 private _locked;
    uint256 private _flashAmount;
    address private _flashToken;
    uint256 private _gasSnapshot;

    bytes32 public constant CALLBACK_SUCCESS = keccak256("ERC3156FlashBorrower.onFlashLoan");

    event FlashMintInitiated(uint256 amount, address executor);
    event FlashMintRepaid(uint256 amount, uint256 fee);
    event ArbitrageExecutedWithFlashMint(uint256 profit, uint256 fee, address executor);

    constructor(
        address _owner,
        address _executor,
        uint256 _bribeBps,
        address _flashMinter,
        address _daiJoin
    ) FlashTipping(_owner, _executor, _bribeBps) {
        require(_flashMinter != address(0), "MW: flashMinter zero");
        require(_daiJoin != address(0), "MW: daiJoin zero");
        flashMinter = IDssFlash(_flashMinter);
        dai = IDai(IDaiJoin(_daiJoin).dai());
        vat = IDaiJoin(_daiJoin).vat();
    }

    function executeWithFlashMint(
        address target,
        bytes calldata arbCalldata,
        uint256 flashAmount
    ) external onlyExecutor returns (bool) {
        require(_locked == 0, "MW: reentrant");
        _locked = 1;
        _flashAmount = flashAmount;
        _gasSnapshot = gasleft();

        flashMinter.flashLoan(
            IERC3156FlashBorrower(address(this)),
            address(dai),
            flashAmount,
            abi.encode(target, arbCalldata)
        );

        _locked = 0;
        return true;
    }

    function onFlashLoan(
        address initiator,
        address token,
        uint256 amount,
        uint256 fee,
        bytes calldata data
    ) external returns (bytes32) {
        require(msg.sender == address(flashMinter), "MW: not flashMinter");
        require(initiator == address(this), "MW: not initiator");
        require(token == address(dai), "MW: not dai");

        (address target, bytes memory arbCalldata) = abi.decode(data, (address, bytes));

        uint256 daiBal = dai.balanceOf(address(this));
        require(daiBal >= amount, "MW: insufficient DAI received");

        emit FlashMintInitiated(amount, address(this));

        (bool success,) = target.call(arbCalldata);
        require(success, "MW: arb call failed");

        uint256 repayAmount = amount + fee;
        uint256 postBal = dai.balanceOf(address(this));
        require(postBal >= repayAmount, "MW: insufficient DAI for repayment");

        if (postBal > repayAmount) {
            uint256 surplus = postBal - repayAmount;
            uint256 gasSpent = (_gasSnapshot - gasleft()) * tx.gasprice;
            (uint256 tip,) = _calculateTip(surplus, gasSpent);
            if (tip > 0 && address(this).balance >= tip) {
                _sendTip(tip);
            }
        }

        dai.transfer(address(flashMinter), repayAmount);

        emit FlashMintRepaid(amount, fee);
        emit ArbitrageExecutedWithFlashMint(
            daiBal > repayAmount ? daiBal - repayAmount : 0,
            fee,
            address(this)
        );

        return CALLBACK_SUCCESS;
    }

    function maxFlashLoan() external view returns (uint256) {
        return flashMinter.maxFlashLoan(address(dai));
    }

    function flashFee(uint256 amount) external view returns (uint256) {
        return flashMinter.flashFee(address(dai), amount);
    }

    function rescueTokens(address token, uint256 amount) external onlyOwner {
        if (token == address(0)) {
            (bool sent,) = payable(owner).call{value: amount}("");
            require(sent, "MW: rescue failed");
        } else {
            IDai(token).transfer(owner, amount);
        }
    }

    receive() external payable {}
}
