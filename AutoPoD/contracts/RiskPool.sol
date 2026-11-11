// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title RiskPool
/// @notice Manages liquidity for autonomous insurance payouts
contract RiskPool {
    address public immutable governance;

    struct Policy {
        address insured;
        uint256 premiumPaid;
        uint256 coverageAmount;
        uint256 activeUntil;
        bool exists;
    }

    mapping(bytes32 => Policy) public policies;
    uint256 public totalLiquidity;
    uint256 public totalPremiums;

    event PolicyCreated(bytes32 indexed policyId, address indexed insured, uint256 coverage, uint256 duration);
    event LiquidityAdded(address indexed provider, uint256 amount);
    event PayoutReleased(bytes32 indexed policyId, address indexed recipient, uint256 amount);

    modifier onlyGovernance() {
        require(msg.sender == governance, "not governance");
        _;
    }

    constructor() {
        governance = msg.sender;
    }

    function createPolicy(
        bytes32 policyId,
        address insured,
        uint256 coverageAmount,
        uint256 durationSeconds
    ) external payable {
        require(!policies[policyId].exists, "policy exists");
        require(msg.value > 0, "premium required");
        require(coverageAmount > 0, "coverage required");

        policies[policyId] = Policy({
            insured: insured,
            premiumPaid: msg.value,
            coverageAmount: coverageAmount,
            activeUntil: block.timestamp + durationSeconds,
            exists: true
        });

        totalPremiums += msg.value;
        totalLiquidity += msg.value;

        emit PolicyCreated(policyId, insured, coverageAmount, durationSeconds);
    }

    function addLiquidity() external payable {
        require(msg.value > 0, "amount required");
        totalLiquidity += msg.value;
        emit LiquidityAdded(msg.sender, msg.value);
    }

    function releasePayout(bytes32 policyId, address recipient, uint256 amount) external onlyGovernance {
        Policy storage policy = policies[policyId];
        require(policy.exists, "policy not found");
        require(block.timestamp <= policy.activeUntil, "policy expired");
        require(amount <= policy.coverageAmount, "exceeds coverage");
        require(amount <= totalLiquidity, "insufficient liquidity");

        policy.coverageAmount -= amount;
        totalLiquidity -= amount;

        (bool sent, ) = payable(recipient).call{value: amount}("");
        require(sent, "transfer failed");

        emit PayoutReleased(policyId, recipient, amount);
    }

    function withdrawLiquidity(uint256 amount) external onlyGovernance {
        require(amount <= totalLiquidity, "insufficient liquidity");
        totalLiquidity -= amount;
        (bool sent, ) = payable(governance).call{value: amount}("");
        require(sent, "transfer failed");
    }

    function getPolicy(bytes32 policyId) external view returns (Policy memory) {
        return policies[policyId];
    }
}
