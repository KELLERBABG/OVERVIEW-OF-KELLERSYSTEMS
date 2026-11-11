// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @title ZK Genome Payment Contract
/// @notice Handles ERC-20 micropayments for valid proof submissions.
///         Pharma funds queries → users submit proofs → automated settlement.
contract Payment {
    /// The token used for payments (USDC or ZKG token).
    address public paymentToken;

    /// Total value locked in pending queries.
    uint256 public totalValueLocked;

    /// A payment escrow for a specific query.
    struct Escrow {
        bytes32 queryId;
        address pharma;
        uint256 amount;
        uint256 rewardPerProof;
        uint256 maxProofs;
        uint256 proofsSubmitted;
        bool active;
    }

    /// All escrows indexed by query ID.
    mapping(bytes32 => Escrow) public escrows;

    /// Tracks which proofs have been paid out (proofHash => paid).
    mapping(bytes32 => bool) public paidProofs;

    event EscrowCreated(bytes32 indexed queryId, address indexed pharma, uint256 amount, uint256 rewardPerProof);
    event ProofPaid(bytes32 indexed proofHash, address indexed user, uint256 amount);
    event EscrowClosed(bytes32 indexed queryId, uint256 remaining);

    constructor(address _paymentToken) {
        paymentToken = _paymentToken;
    }

    /// Pharma creates an escrow for a query with a reward budget.
    function createEscrow(
        bytes32 queryId,
        uint256 totalBudget,
        uint256 _rewardPerProof,
        uint256 _maxProofs
    ) external {
        require(totalBudget > 0, "Budget must be > 0");
        require(_rewardPerProof > 0, "Reward must be > 0");
        require(totalBudget >= _rewardPerProof * _maxProofs, "Budget insufficient");
        require(escrows[queryId].pharma == address(0), "Query already exists");

        escrows[queryId] = Escrow({
            queryId: queryId,
            pharma: msg.sender,
            amount: totalBudget,
            rewardPerProof: _rewardPerProof,
            maxProofs: _maxProofs,
            proofsSubmitted: 0,
            active: true
        });

        totalValueLocked += totalBudget;
        emit EscrowCreated(queryId, msg.sender, totalBudget, _rewardPerProof);
    }

    /// Submit a valid proof hash and claim reward.
    function submitProof(bytes32 queryId, bytes32 proofHash, address user) external {
        Escrow storage escrow = escrows[queryId];
        require(escrow.active, "Escrow not active");
        require(escrow.proofsSubmitted < escrow.maxProofs, "Max proofs reached");
        require(!paidProofs[proofHash], "Proof already paid");

        paidProofs[proofHash] = true;
        escrow.proofsSubmitted++;

        // Transfer reward (in production: IERC20(paymentToken).transfer(user, escrow.rewardPerProof))
        emit ProofPaid(proofHash, user, escrow.rewardPerProof);
    }

    /// Pharma closes the escrow and reclaims remaining funds.
    function closeEscrow(bytes32 queryId) external {
        Escrow storage escrow = escrows[queryId];
        require(msg.sender == escrow.pharma, "Only pharma can close");
        require(escrow.active, "Already closed");

        escrow.active = false;
        uint256 remaining = escrow.amount - (escrow.rewardPerProof * escrow.proofsSubmitted);
        totalValueLocked -= escrow.amount;

        emit EscrowClosed(queryId, remaining);
    }

    /// View escrow details.
    function getEscrow(bytes32 queryId) external view returns (Escrow memory) {
        return escrows[queryId];
    }
}