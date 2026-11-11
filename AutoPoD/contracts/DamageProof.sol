// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./RiskPool.sol";

/// @title DamageProof
/// @notice Verifies damage reports and triggers autonomous payouts
contract DamageProof {
    RiskPool public immutable riskPool;
    address public immutable verifier;

    struct DamageReport {
        bytes32 reportHash;
        bool verified;
        bool paid;
        uint256 payoutAmount;
        uint256 timestamp;
    }

    mapping(bytes32 => DamageReport) public reports;
    mapping(bytes32 => bool) public usedSignatures;

    event ReportSubmitted(bytes32 indexed reportId, bytes32 indexed policyId, bytes32 reportHash);
    event ReportVerified(bytes32 indexed reportId, bool approved, string classification);
    event PayoutTriggered(bytes32 indexed reportId, address indexed recipient, uint256 amount);

    modifier onlyVerifier() {
        require(msg.sender == verifier, "not verifier");
        _;
    }

    constructor(address _riskPool) {
        require(_riskPool != address(0), "invalid pool");
        riskPool = RiskPool(_riskPool);
        verifier = msg.sender;
    }

    /// @notice Submit a damage report hash for verification
    function submitReport(bytes32 reportId, bytes32 policyId, bytes32 reportHash) external {
        require(reports[reportId].timestamp == 0, "report exists");
        reports[reportId] = DamageReport({
            reportHash: reportHash,
            verified: false,
            paid: false,
            payoutAmount: 0,
            timestamp: block.timestamp
        });
        emit ReportSubmitted(reportId, policyId, reportHash);
    }

    /// @notice Verify a damage report and trigger payout if approved
    /// @dev This is the core invariant: once verified, payout is immediate and unstoppable
    function verifyAndPayout(
        bytes32 reportId,
        bytes32 policyId,
        bytes32 reportHash,
        address insured,
        uint256 payoutAmount,
        string memory classification,
        bytes32 signature
    ) external onlyVerifier {
        require(!usedSignatures[signature], "signature replayed");
        usedSignatures[signature] = true;

        DamageReport storage report = reports[reportId];
        require(report.timestamp > 0, "report not found");
        require(!report.verified, "already verified");
        require(report.reportHash == reportHash, "hash mismatch");
        require(payoutAmount > 0, "zero payout");

        report.verified = true;
        report.payoutAmount = payoutAmount;

        emit ReportVerified(reportId, true, classification);

        bool approved = keccak256(bytes(classification)) != keccak256(bytes("REJECTED"));
        if (approved) {
            report.paid = true;
            riskPool.releasePayout(policyId, insured, payoutAmount);
            emit PayoutTriggered(reportId, insured, payoutAmount);
        }
    }

    /// @notice Get report details
    function getReport(bytes32 reportId) external view returns (DamageReport memory) {
        return reports[reportId];
    }
}
